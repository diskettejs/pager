use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannel, FifoChannelHandler, RingChannel, RingChannelHandler};
use zenoh::key_expr::KeyExpr as ZKeyExpr;
use zenoh::liveliness::LivelinessToken as ZLivelinessToken;
use zenoh::pubsub::Subscriber as ZSubscriber;
use zenoh::sample::Sample as ZSample;
use zenoh::session::EntityGlobalId as ZEntityGlobalId;

use crate::handlers::{
  ChannelKind, DEFAULT_CHANNEL_CAPACITY, FifoChannelHandlerReply, FifoChannelHandlerSample,
  RingChannelHandlerReply, RingChannelHandlerSample,
};
use crate::keyexpr::{KeyExpr, KeyExprArg};
use crate::options::{LivelinessGetOptions, LivelinessSubscriberOptions};
use crate::session::EntityGlobalId;

#[napi]
pub struct Liveliness {
  session: zenoh::Session,
}

impl Liveliness {
  pub(crate) fn from_session(session: zenoh::Session) -> Self {
    Liveliness { session }
  }
}

#[napi]
impl Liveliness {
  /// Declares a liveliness token on `keyExpr`. The token asserts this session's
  /// liveliness for that key expression until it is undeclared or dropped.
  #[napi]
  pub async fn declare_token(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
  ) -> napi::Result<LivelinessToken> {
    let liveliness = self.session.liveliness();
    let token = liveliness
      .declare_token(key_expr.0)
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(LivelinessToken::from_inner(token))
  }

  /// Declares a subscription to liveliness changes matching `keyExpr`.
  #[napi]
  pub async fn declare_subscriber(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<LivelinessSubscriberOptions>,
  ) -> napi::Result<LivelinessSubscriber> {
    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));
    let history = options.as_ref().and_then(|o| o.history).unwrap_or(false);

    let liveliness = self.session.liveliness();
    let mut builder = liveliness.declare_subscriber(key_expr.0);
    if history {
      builder = builder.history(true);
    }

    if is_ring {
      let sub = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = sub.key_expr().clone();
      let id = sub.id();
      Ok(LivelinessSubscriber::from_ring(sub, key_expr, id))
    } else {
      let sub = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = sub.key_expr().clone();
      let id = sub.id();
      Ok(LivelinessSubscriber::from_fifo(sub, key_expr, id))
    }
  }

  /// Queries liveliness tokens matching `keyExpr` and returns the reply handler.
  /// The handler completes (disconnects) once the query is resolved.
  #[napi]
  pub async fn get(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<LivelinessGetOptions>,
  ) -> napi::Result<Either<FifoChannelHandlerReply, RingChannelHandlerReply>> {
    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    let liveliness = self.session.liveliness();
    let mut builder = liveliness.get(key_expr.0);
    if let Some(opts) = options {
      if let Some(timeout_ms) = opts.timeout {
        builder = builder.timeout(Duration::from_millis(timeout_ms as u64));
      }
      if let Some(cancellation_token) = opts.cancellation_token {
        builder = builder.cancellation_token(cancellation_token.0);
      }
    }

    if is_ring {
      let handler = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(Either::B(RingChannelHandlerReply::from_arc(Arc::new(
        handler,
      ))))
    } else {
      let handler = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(Either::A(FifoChannelHandlerReply::from_handler(handler)))
    }
  }
}

#[napi]
pub struct LivelinessToken {
  pub(crate) inner: Option<ZLivelinessToken>,
}

impl LivelinessToken {
  pub(crate) fn from_inner(inner: ZLivelinessToken) -> Self {
    LivelinessToken { inner: Some(inner) }
  }
}

#[napi]
impl LivelinessToken {
  /// Undeclare this liveliness token. If the token was already undeclared (or
  /// dropped), this is a no-op.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(token) => token
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      None => Ok(()),
    }
  }
}

enum SubInner {
  Fifo(ZSubscriber<FifoChannelHandler<ZSample>>),
  Ring(Arc<ZSubscriber<RingChannelHandler<ZSample>>>),
}

/// A subscription to liveliness changes on a key expression.
#[napi]
pub struct LivelinessSubscriber {
  // `None` once undeclared. `key_expr`/`id` are cached so they survive it.
  inner: Option<SubInner>,
  key_expr: ZKeyExpr<'static>,
  id: ZEntityGlobalId,
}

impl LivelinessSubscriber {
  pub(crate) fn from_fifo(
    sub: ZSubscriber<FifoChannelHandler<ZSample>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    LivelinessSubscriber {
      inner: Some(SubInner::Fifo(sub)),
      key_expr,
      id,
    }
  }

  pub(crate) fn from_ring(
    sub: ZSubscriber<RingChannelHandler<ZSample>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    LivelinessSubscriber {
      inner: Some(SubInner::Ring(Arc::new(sub))),
      key_expr,
      id,
    }
  }
}

#[napi]
impl LivelinessSubscriber {
  /// The key expression this subscription matches.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.key_expr.clone())
  }

  /// The global id of this subscription entity.
  #[napi(getter)]
  pub fn id(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(self.id)
  }

  /// The receive end of the subscription. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at declare time.
  #[napi(getter)]
  pub fn handler(
    &self,
  ) -> napi::Result<Either<FifoChannelHandlerSample, RingChannelHandlerSample>> {
    match self.inner.as_ref() {
      Some(SubInner::Fifo(sub)) => Ok(Either::A(FifoChannelHandlerSample::from_handler(
        sub.handler().clone(),
      ))),
      Some(SubInner::Ring(arc)) => Ok(Either::B(RingChannelHandlerSample::from_arc(Arc::clone(
        arc,
      )))),
      None => Err(napi::Error::from_reason(
        "liveliness subscriber has been undeclared",
      )),
    }
  }

  /// Undeclare this subscription. Resolves once undeclaration completes; a
  /// second call is a no-op.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(SubInner::Fifo(sub)) => sub
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      Some(SubInner::Ring(arc)) => match Arc::try_unwrap(arc) {
        Ok(sub) => sub
          .undeclare()
          .await
          .map_err(|e| napi::Error::from_reason(e.to_string())),
        Err(_) => Ok(()),
      },
      None => Ok(()),
    }
  }
}
