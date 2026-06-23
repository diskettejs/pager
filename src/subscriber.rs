use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannel, FifoChannelHandler, RingChannel, RingChannelHandler};
use zenoh::key_expr::KeyExpr as ZKeyExpr;
use zenoh::sample::Sample as ZSample;
use zenoh::session::EntityGlobalId as ZEntityGlobalId;
use zenoh_ext::AdvancedSubscriber;

use crate::entity_global_id::EntityGlobalId;
use crate::handlers::{
  ChannelKind, DEFAULT_CHANNEL_CAPACITY, FifoChannelHandlerSample, RingChannelHandlerSample,
};
use crate::keyexpr::KeyExpr;
use crate::liveliness_subscriber::LivelinessSubscriber;
use crate::options::{LivelinessSubscriberOptions, SampleMissListenerOptions};
use crate::sample_miss_listener::SampleMissListener;

enum SubInner {
  Fifo(AdvancedSubscriber<FifoChannelHandler<ZSample>>),
  Ring(Arc<AdvancedSubscriber<RingChannelHandler<ZSample>>>),
}

#[napi]
pub struct Subscriber {
  // `None` once undeclared. `key_expr`/`id` are cached so they survive it.
  inner: Option<SubInner>,
  key_expr: ZKeyExpr<'static>,
  id: ZEntityGlobalId,
}

impl Subscriber {
  pub(crate) fn from_fifo(
    sub: AdvancedSubscriber<FifoChannelHandler<ZSample>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    Subscriber {
      inner: Some(SubInner::Fifo(sub)),
      key_expr,
      id,
    }
  }

  pub(crate) fn from_ring(
    sub: AdvancedSubscriber<RingChannelHandler<ZSample>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    Subscriber {
      inner: Some(SubInner::Ring(Arc::new(sub))),
      key_expr,
      id,
    }
  }
}

#[napi]
impl Subscriber {
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
  ///
  /// The handler is not iterable; iterate via `subscriber.handler.stream()`.
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
      None => Err(napi::Error::from_reason("subscriber has been undeclared")),
    }
  }

  /// Declares a listener that notifies of samples missed on this subscription.
  ///
  /// Misses are only detected when the matching publisher enables
  /// `sampleMissDetection`. The `handler` option chooses the channel (default:
  /// FIFO of [`DEFAULT_CHANNEL_CAPACITY`]); it is independent of the
  /// subscription's own channel.
  #[napi]
  pub async fn sample_miss_listener(
    &self,
    options: Option<SampleMissListenerOptions>,
  ) -> napi::Result<SampleMissListener> {
    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    // The builder type (`SampleMissListenerBuilder<'_, DefaultHandler>`) is the
    // same for both subscription channels, so unify before picking the listener
    // channel below.
    let builder = match self.inner.as_ref() {
      Some(SubInner::Fifo(sub)) => sub.sample_miss_listener(),
      Some(SubInner::Ring(arc)) => arc.sample_miss_listener(),
      None => return Err(napi::Error::from_reason("subscriber has been undeclared")),
    };

    if is_ring {
      let listener = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(SampleMissListener::from_ring(listener))
    } else {
      let listener = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(SampleMissListener::from_fifo(listener))
    }
  }

  /// Declares a liveliness subscription that detects publishers matching this
  /// subscription's key expression.
  ///
  /// Only publishers that enable `publisherDetection` are detectable. Resolves
  /// to a `LivelinessSubscriber` over the derived detection key expression (a
  /// `Put` marks a publisher appearing, a `Delete` one disappearing). The
  /// `handler` option chooses the channel (default: FIFO of
  /// [`DEFAULT_CHANNEL_CAPACITY`]); `history` replays the currently-matching
  /// publishers on declaration.
  #[napi]
  pub async fn detect_publishers(
    &self,
    options: Option<LivelinessSubscriberOptions>,
  ) -> napi::Result<LivelinessSubscriber> {
    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));
    let history = options.as_ref().and_then(|o| o.history).unwrap_or(false);

    // The builder type (`LivelinessSubscriberBuilder<'_, '_, DefaultHandler>`) is
    // the same for both subscription channels, so unify before picking the
    // detection subscriber's channel below.
    let mut builder = match self.inner.as_ref() {
      Some(SubInner::Fifo(sub)) => sub.detect_publishers(),
      Some(SubInner::Ring(arc)) => arc.detect_publishers(),
      None => return Err(napi::Error::from_reason("subscriber has been undeclared")),
    };
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

  /// Undeclare this subscription. Resolves once undeclaration completes; a
  /// second call is a no-op.
  ///
  /// For a ring subscription still referenced by an outstanding handler, this
  /// drops our strong reference and lets the background drop undeclare it once
  /// the last handler is released.
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
