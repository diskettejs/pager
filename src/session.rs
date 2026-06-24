use std::sync::Arc;
use std::time::Duration;

use napi::bindgen_prelude::{BigInt, Either, Uint8Array};
use napi_derive::napi;
use zenoh::bytes::ZBytes;
use zenoh::handlers::{FifoChannel, RingChannel};
use zenoh::query::ConsolidationMode as ZConsolidationMode;
use zenoh::session::EntityGlobalId as ZEntityGlobalId;
use zenoh_ext::{AdvancedPublisherBuilderExt, AdvancedSubscriberBuilderExt};

use crate::config::Config;
use crate::handlers::{
  ChannelKind, DEFAULT_CHANNEL_CAPACITY, FifoChannelHandlerReply, RingChannelHandlerReply,
};
use crate::info::SessionInfo;
use crate::keyexpr::{KeyExpr, KeyExprArg};
use crate::liveliness::Liveliness;
use crate::options::{
  DeleteOptions, GetOptions, PublisherOptions, PutOptions, QuerierOptions, QueryableOptions,
  SubscriberOptions, recovery_into_zenoh,
};
use crate::publisher::Publisher;
use crate::querier::Querier;
use crate::queryable::Queryable;
use crate::selector::SelectorArg;
use crate::subscriber::Subscriber;
use crate::time::Timestamp;

#[napi]
pub struct EntityGlobalId {
  pub(crate) inner: ZEntityGlobalId,
}

impl EntityGlobalId {
  pub(crate) fn from_inner(inner: ZEntityGlobalId) -> Self {
    EntityGlobalId { inner }
  }
}

#[napi]
impl EntityGlobalId {
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  #[napi(getter)]
  pub fn eid(&self) -> u32 {
    self.inner.eid()
  }
}

#[napi]
pub struct Session {
  inner: zenoh::Session,
}

impl Session {
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

  /// The global id of this session entity.
  #[napi(getter)]
  pub fn id(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(self.inner.id())
  }

  /// Whether the session has been closed.
  #[napi(getter)]
  pub fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }

  /// Mints a new timestamp from the session's clock, stamped with this
  /// session's Zenoh id. A fresh value is produced on each call.
  #[napi]
  pub fn new_timestamp(&self) -> Timestamp {
    Timestamp::from_inner(self.inner.new_timestamp())
  }

  /// The liveliness sub-API for this session (tokens, subscribers, get).
  #[napi]
  pub fn liveliness(&self) -> Liveliness {
    Liveliness::from_session(self.inner.clone())
  }

  /// The connectivity info sub-API for this session (transports, links, their
  /// Zenoh ids, and lifecycle-event listeners).
  #[napi]
  pub fn info(&self) -> SessionInfo {
    SessionInfo::from_session(self.inner.clone())
  }

  /// The live configuration sub-API for this session: read current values with
  /// `get` / `getPluginConfig`, reconfigure the running session with
  /// `insertJson5`.
  #[napi]
  pub fn config(&self) -> SessionConfig {
    SessionConfig::from_session(self.inner.clone())
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

  /// Deletes the data matching `keyExpr` (publishes a `Delete` sample).
  ///
  /// A shortcut for declaring a publisher and calling `delete` on it.
  #[napi]
  pub async fn delete(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<DeleteOptions>,
  ) -> napi::Result<()> {
    let session = self.inner.clone();
    let ke = key_expr.0;

    let mut builder = session.delete(ke);
    if let Some(opts) = options {
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

  /// Declares `keyExpr` on the session, returning an optimized handle to it.
  #[napi]
  pub async fn declare_keyexpr(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
  ) -> napi::Result<KeyExpr> {
    let session = self.inner.clone();
    let declared = session
      .declare_keyexpr(key_expr.0)
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(KeyExpr::from_inner(declared))
  }

  /// Declares a subscription on `keyExpr`.
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

  /// Declares a queryable on `keyExpr` that answers matching queries.
  #[napi]
  pub async fn declare_queryable(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<QueryableOptions>,
  ) -> napi::Result<Queryable> {
    let session = self.inner.clone();
    let ke = key_expr.0;

    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    let mut builder = session.declare_queryable(ke);
    if let Some(opts) = options {
      if let Some(complete) = opts.complete {
        builder = builder.complete(complete);
      }
      if let Some(allowed_origin) = opts.allowed_origin {
        builder = builder.allowed_origin(allowed_origin.into());
      }
    }

    if is_ring {
      let queryable = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = queryable.key_expr().clone();
      let id = queryable.id();
      Ok(Queryable::from_ring(queryable, key_expr, id))
    } else {
      let queryable = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = queryable.key_expr().clone();
      let id = queryable.id();
      Ok(Queryable::from_fifo(queryable, key_expr, id))
    }
  }

  /// Sends a one-shot query on `selector` and returns the reply handler.
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

/// Distinct from the `Config` used to *open* a session
/// (an owned, pre-open snapshot): this is a live handle onto the running
/// session's configuration. `get` / `getPluginConfig` read the current values
/// and `insertJson5` reconfigures the session in place.
#[napi]
pub struct SessionConfig {
  session: zenoh::Session,
}

impl SessionConfig {
  pub(crate) fn from_session(session: zenoh::Session) -> Self {
    SessionConfig { session }
  }
}

#[napi]
impl SessionConfig {
  /// Reads the configuration value at `key`, returned as a JSON string.
  #[napi]
  pub fn get(&self, key: String) -> napi::Result<String> {
    self
      .session
      .config()
      .get(&key)
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Inserts the JSON5 `value` at `key`, reconfiguring the running session.
  #[napi]
  pub fn insert_json5(&self, key: String, value: String) -> napi::Result<()> {
    self
      .session
      .config()
      .insert_json5(&key, &value)
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// The full current configuration, as a JSON string.
  #[napi]
  pub fn to_json(&self) -> String {
    self.session.config().to_json()
  }

  /// The default timeout applied to queries, in milliseconds.
  #[napi]
  pub fn queries_default_timeout_ms(&self) -> BigInt {
    BigInt::from(self.session.config().queries_default_timeout_ms())
  }

  /// Reads the configuration of plugin `pluginName`, returned as a JSON string.
  #[napi]
  pub fn get_plugin_config(&self, plugin_name: String) -> napi::Result<String> {
    self
      .session
      .config()
      .get_plugin_config(&plugin_name)
      .map(|value| value.to_string())
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }
}
