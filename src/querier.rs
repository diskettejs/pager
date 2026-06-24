use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannel, RingChannel};
use zenoh::key_expr::KeyExpr as ZKeyExpr;
use zenoh::qos::{CongestionControl as ZCongestionControl, Priority as ZPriority};
use zenoh::query::{Querier as ZQuerier, ReplyKeyExpr as ZReplyKeyExpr};
use zenoh::session::EntityGlobalId as ZEntityGlobalId;

use crate::handlers::{
  ChannelKind, DEFAULT_CHANNEL_CAPACITY, FifoChannelHandlerReply, RingChannelHandlerReply,
};
use crate::keyexpr::KeyExpr;
use crate::matching::{MatchingListener, MatchingStatus};
use crate::options::{MatchingListenerOptions, QuerierGetOptions};
use crate::qos::{CongestionControl, Priority};
use crate::query::ReplyKeyExpr;
use crate::session::EntityGlobalId;

#[napi]
pub struct Querier {
  // `None` once undeclared. The cached config below survives it.
  inner: Option<ZQuerier<'static>>,
  key_expr: ZKeyExpr<'static>,
  id: ZEntityGlobalId,
  congestion_control: ZCongestionControl,
  priority: ZPriority,
  accept_replies: ZReplyKeyExpr,
}

impl Querier {
  pub(crate) fn from_inner(querier: ZQuerier<'static>) -> Self {
    let key_expr = querier.key_expr().clone();
    let id = querier.id();
    let congestion_control = querier.congestion_control();
    let priority = querier.priority();
    let accept_replies = querier.accept_replies();
    Querier {
      inner: Some(querier),
      key_expr,
      id,
      congestion_control,
      priority,
      accept_replies,
    }
  }
}

#[napi]
impl Querier {
  /// The key expression this querier sends queries on.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.key_expr.clone())
  }

  /// The global id of this querier entity.
  #[napi(getter)]
  pub fn id(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(self.id)
  }

  /// The congestion control applied when routing this querier's queries.
  #[napi(getter)]
  pub fn congestion_control(&self) -> CongestionControl {
    self.congestion_control.into()
  }

  /// The priority of this querier's queries.
  #[napi(getter)]
  pub fn priority(&self) -> Priority {
    self.priority.into()
  }

  /// Whether this querier accepts replies whose key expression doesn't match
  /// the query.
  #[napi(getter)]
  pub fn accept_replies(&self) -> ReplyKeyExpr {
    self.accept_replies.into()
  }

  /// Sends a query and returns the reply handler. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen via the `handler`
  /// option (default: FIFO with capacity 256). Completes (disconnects) once the query is resolved.
  #[napi]
  pub async fn get(
    &self,
    options: Option<QuerierGetOptions>,
  ) -> napi::Result<Either<FifoChannelHandlerReply, RingChannelHandlerReply>> {
    let querier = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("querier has been undeclared"))?;

    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    let mut builder = querier.get();
    if let Some(opts) = options {
      if let Some(parameters) = opts.parameters {
        builder = builder.parameters(parameters.0);
      }
      if let Some(payload) = opts.payload {
        builder = builder.payload(payload.to_vec());
      }
      if let Some(encoding) = opts.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(attachment) = opts.attachment {
        builder = builder.attachment(attachment.to_vec());
      }
      if let Some(source_info) = opts.source_info {
        builder = builder.source_info(source_info.0);
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

  /// The current matching status of this querier — whether any queryables match
  /// its key expression and target.
  #[napi]
  pub async fn matching_status(&self) -> napi::Result<MatchingStatus> {
    let querier = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("querier has been undeclared"))?;
    let status = querier
      .matching_status()
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(MatchingStatus::from_inner(status))
  }

  /// Declares a listener that notifies whenever this querier's matching status
  /// changes (matching queryables appear or disappear).
  #[napi]
  pub async fn matching_listener(
    &self,
    options: Option<MatchingListenerOptions>,
  ) -> napi::Result<MatchingListener> {
    let querier = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("querier has been undeclared"))?;

    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    if is_ring {
      let listener = querier
        .matching_listener()
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(MatchingListener::from_ring(listener))
    } else {
      let listener = querier
        .matching_listener()
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(MatchingListener::from_fifo(listener))
    }
  }

  /// Undeclare this querier. Resolves once undeclaration completes; a second
  /// call is a no-op.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(querier) => querier
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      None => Ok(()),
    }
  }
}
