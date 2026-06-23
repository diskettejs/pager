use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::{Either, Uint8Array};
use napi_derive::napi;
use zenoh::bytes::ZBytes;
use zenoh::handlers::{FifoChannel, RingChannel};
use zenoh::query::ConsolidationMode as ZConsolidationMode;
use zenoh_ext::{AdvancedPublisherBuilderExt, AdvancedSubscriberBuilderExt};

use crate::config::Config;
use crate::handlers::{
  ChannelKind, DEFAULT_CHANNEL_CAPACITY, FifoChannelHandlerReply, RingChannelHandlerReply,
};
use crate::keyexpr::KeyExprArg;
use crate::liveliness::Liveliness;
use crate::options::{
  GetOptions, PublisherOptions, PutOptions, QuerierOptions, SubscriberOptions, recovery_into_zenoh,
};
use crate::publisher::Publisher;
use crate::querier::Querier;
use crate::selector::SelectorArg;
use crate::subscriber::Subscriber;

#[napi]
pub struct Session {
  inner: zenoh::Session,
}

impl Session {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: zenoh::Session) -> Self {
    Session { inner }
  }
}

#[napi]
impl Session {
  /// Opens a session with the given configuration.
  #[napi(factory)]
  pub async fn open(config: &Config) -> napi::Result<Session> {
    let cfg = config.inner.clone();
    let session = zenoh::open(cfg)
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Session::from_inner(session))
  }

  /// This session's Zenoh id, as a hex string.
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  /// Whether the session has been closed.
  #[napi(getter)]
  pub fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }

  /// The liveliness sub-API for this session (tokens, subscribers, get).
  #[napi]
  pub fn liveliness(&self) -> Liveliness {
    Liveliness::from_session(self.inner.clone())
  }

  /// Closes the session, undeclaring everything declared on it.
  #[napi]
  pub async fn close(&self) -> napi::Result<()> {
    let session = self.inner.clone();
    session
      .close()
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Publishes `payload` on `keyExpr`.
  #[napi]
  pub async fn put(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    payload: Either<String, Uint8Array>,
    options: Option<PutOptions>,
  ) -> napi::Result<()> {
    let session = self.inner.clone();
    let ke = key_expr.0;
    let payload: ZBytes = match payload {
      Either::A(s) => ZBytes::from(s),
      Either::B(bytes) => ZBytes::from(bytes.to_vec()),
    };

    let mut builder = session.put(ke, payload);
    if let Some(opts) = options {
      if let Some(encoding) = opts.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(congestion_control) = opts.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = opts.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = opts.express {
        builder = builder.express(express);
      }
      if let Some(reliability) = opts.reliability {
        builder = builder.reliability(reliability.into());
      }
      if let Some(allowed_destination) = opts.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timestamp) = opts.timestamp {
        builder = builder.timestamp(timestamp.0);
      }
      if let Some(attachment) = opts.attachment {
        builder = builder.attachment(attachment.to_vec());
      }
      if let Some(source_info) = opts.source_info {
        builder = builder.source_info(source_info.0);
      }
    }

    builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Declares a subscription on `keyExpr`.
  ///
  /// The `handler` option chooses the channel (default: FIFO of
  /// [`DEFAULT_CHANNEL_CAPACITY`]). Advanced options (`allowedOrigin`,
  /// `history`, `recovery`, subscriber detection, `queryTimeoutMs`) are applied
  /// to the advanced builder.
  #[napi]
  pub async fn declare_subscriber(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<SubscriberOptions>,
  ) -> napi::Result<Subscriber> {
    let session = self.inner.clone();
    let ke = key_expr.0;

    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    // Advanced options are applied to the `.advanced()` builder before the
    // channel is chosen (they're generic over the handler), so the builder is
    // built once and only `.with()` differs per channel below.
    let mut builder = session.declare_subscriber(ke).advanced();
    if let Some(opts) = options {
      if let Some(allowed_origin) = opts.allowed_origin {
        builder = builder.allowed_origin(allowed_origin.into());
      }
      if let Some(history) = opts.history {
        builder = builder.history(history.into_zenoh());
      }
      if let Some(recovery) = opts.recovery {
        builder = builder.recovery(recovery_into_zenoh(recovery));
      }
      if opts.subscriber_detection == Some(true) {
        builder = builder.subscriber_detection();
      }
      if let Some(metadata) = opts.subscriber_detection_metadata {
        builder = builder.subscriber_detection_metadata(metadata);
      }
      if let Some(query_timeout_ms) = opts.query_timeout_ms {
        builder = builder.query_timeout(Duration::from_millis(query_timeout_ms as u64));
      }
    }

    if is_ring {
      let sub = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = sub.key_expr().clone();
      let id = sub.id();
      Ok(Subscriber::from_ring(sub, key_expr, id))
    } else {
      let sub = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = sub.key_expr().clone();
      let id = sub.id();
      Ok(Subscriber::from_fifo(sub, key_expr, id))
    }
  }

  /// Declares a publisher on `keyExpr`, fixing its QoS for every publication.
  ///
  /// Advanced options (`cache`, `sampleMissDetection`, publisher detection) are
  /// applied to the advanced builder.
  #[napi]
  pub async fn declare_publisher(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<PublisherOptions>,
  ) -> napi::Result<Publisher> {
    let session = self.inner.clone();
    let ke = key_expr.0;

    let mut builder = session.declare_publisher(ke).advanced();
    if let Some(opts) = options {
      if let Some(encoding) = opts.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(congestion_control) = opts.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = opts.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = opts.express {
        builder = builder.express(express);
      }
      if let Some(reliability) = opts.reliability {
        builder = builder.reliability(reliability.into());
      }
      if let Some(allowed_destination) = opts.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(cache) = opts.cache {
        builder = builder.cache(cache.into_zenoh());
      }
      if let Some(sample_miss_detection) = opts.sample_miss_detection {
        builder = builder.sample_miss_detection(sample_miss_detection.into_zenoh());
      }
      if opts.publisher_detection == Some(true) {
        builder = builder.publisher_detection();
      }
      if let Some(metadata) = opts.publisher_detection_metadata {
        builder = builder.publisher_detection_metadata(metadata);
      }
    }

    let publisher = builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Publisher::from_inner(publisher))
  }

  /// Declares a querier on `keyExpr`, fixing its config for every `get`.
  #[napi]
  pub async fn declare_querier(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<QuerierOptions>,
  ) -> napi::Result<Querier> {
    let session = self.inner.clone();
    let ke = key_expr.0;

    let mut builder = session.declare_querier(ke);
    if let Some(opts) = options {
      if let Some(target) = opts.target {
        builder = builder.target(target.into());
      }
      if let Some(consolidation) = opts.consolidation {
        builder = builder.consolidation(ZConsolidationMode::from(consolidation));
      }
      if let Some(congestion_control) = opts.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = opts.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = opts.express {
        builder = builder.express(express);
      }
      if let Some(allowed_destination) = opts.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timeout_ms) = opts.timeout {
        builder = builder.timeout(Duration::from_millis(timeout_ms as u64));
      }
      if let Some(accept_replies) = opts.accept_replies {
        builder = builder.accept_replies(accept_replies.into());
      }
    }

    let querier = builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Querier::from_inner(querier))
  }

  /// Sends a one-shot query on `selector` and returns the reply handler. A
  /// `FifoChannelHandler` or `RingChannelHandler` depending on the channel
  /// chosen via the `handler` option (default: FIFO of
  /// [`DEFAULT_CHANNEL_CAPACITY`]).
  ///
  /// The selector is a key expression plus optional parameters — pass a
  /// `key/expr?p=1` string, a `KeyExpr` (no parameters), or a `Selector`. The
  /// handler is not iterable; iterate via `replies.stream()`. It completes
  /// (disconnects) once the query is resolved.
  #[napi]
  pub async fn get(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr | Selector")] selector: SelectorArg,
    options: Option<GetOptions>,
  ) -> napi::Result<Either<FifoChannelHandlerReply, RingChannelHandlerReply>> {
    let session = self.inner.clone();

    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    let mut builder = session.get(selector.0);
    if let Some(opts) = options {
      if let Some(target) = opts.target {
        builder = builder.target(target.into());
      }
      if let Some(consolidation) = opts.consolidation {
        builder = builder.consolidation(ZConsolidationMode::from(consolidation));
      }
      if let Some(congestion_control) = opts.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = opts.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = opts.express {
        builder = builder.express(express);
      }
      if let Some(allowed_destination) = opts.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timeout_ms) = opts.timeout {
        builder = builder.timeout(Duration::from_millis(timeout_ms as u64));
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
}
