use std::str::FromStr;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::session::ZenohId;

use crate::bytes::to_zbytes;
use crate::config::Config;
use crate::error::to_napi_err;
use crate::publisher::{Publisher, PublisherOptions};
use crate::qos::{CongestionControl, Priority, Reliability};
use crate::querier::{Querier, QuerierOptions};
use crate::query::{GetOptions, Replies};
use crate::queryable::{Queryable, QueryableOptions};
use crate::sample::{Locality, SourceInfo};
use crate::subscriber::{Subscriber, SubscriberOptions};
use crate::time::Timestamp;

/// An open connection to the Zenoh network — the entry point from which every
/// publisher, subscriber, and query is declared. Close it with `close`.
#[napi]
pub struct Session {
  pub(crate) inner: zenoh::Session,
}

/// Globally-unique identifier of a Zenoh entity (a session, publisher, …): the
/// owning session's Zenoh ID together with a session-local entity id.
#[napi(object)]
#[derive(Clone)]
pub struct EntityGlobalId {
  /// Zenoh ID of the owning session, as a hex string.
  pub zid: String,
  /// Session-local entity id.
  pub eid: u32,
}

impl EntityGlobalId {
  pub(crate) fn from_zenoh(id: zenoh::session::EntityGlobalId) -> Self {
    Self {
      zid: id.zid().to_string(),
      eid: id.eid(),
    }
  }

  pub(crate) fn to_zenoh(&self) -> Result<zenoh::session::EntityGlobalId> {
    Ok(zenoh::session::EntityGlobalId::new(
      ZenohId::from_str(&self.zid).map_err(to_napi_err)?,
      self.eid,
    ))
  }
}

/// Options for [`Session::put`].
#[napi(object)]
pub struct PutOptions {
  /// Encoding of the payload (e.g. `"text/plain"`, `"application/json"`).
  pub encoding: Option<String>,
  /// Optional attachment carried alongside the payload.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// Congestion control strategy (default: `Drop`).
  pub congestion_control: Option<CongestionControl>,
  /// Priority of the publication (default: `Data`).
  pub priority: Option<Priority>,
  /// When `true`, the message is sent unbatched, trading throughput for latency.
  pub express: Option<bool>,
  /// Delivery reliability (default: `Reliable`).
  pub reliability: Option<Reliability>,
  /// Restrict which matching subscribers receive the data (default: `Any`).
  pub allowed_destination: Option<Locality>,
  /// Timestamp to attach; obtain one from [`Session::newTimestamp`].
  pub timestamp: Option<Timestamp>,
  /// Source metadata (producing entity + sequence number).
  pub source_info: Option<SourceInfo>,
}

/// Options for [`Session::delete`].
#[napi(object)]
pub struct DeleteOptions {
  /// Optional attachment carried alongside the deletion.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// Congestion control strategy (default: `Drop`).
  pub congestion_control: Option<CongestionControl>,
  /// Priority of the deletion (default: `Data`).
  pub priority: Option<Priority>,
  /// When `true`, the message is sent unbatched, trading throughput for latency.
  pub express: Option<bool>,
  /// Delivery reliability (default: `Reliable`).
  pub reliability: Option<Reliability>,
  /// Restrict which matching subscribers receive the deletion (default: `Any`).
  pub allowed_destination: Option<Locality>,
  /// Timestamp to attach; obtain one from [`Session::newTimestamp`].
  pub timestamp: Option<Timestamp>,
  /// Source metadata (producing entity + sequence number).
  pub source_info: Option<SourceInfo>,
}

#[napi]
impl Session {
  /// Open a session with the given configuration, or the default configuration
  /// when omitted.
  #[napi(factory)]
  pub async fn open(config: Option<&Config>) -> Result<Self> {
    let config = config.map(|c| c.inner.clone()).unwrap_or_default();
    let inner = zenoh::open(config).await.map_err(to_napi_err)?;
    Ok(Self { inner })
  }

  /// Close the session, undeclaring every entity declared on it.
  #[napi]
  pub async fn close(&self) -> Result<()> {
    self.inner.close().await.map_err(to_napi_err)
  }

  /// Publish a `Put` sample: send `payload` to every subscriber whose key
  /// expression matches `key_expr`.
  #[napi]
  pub async fn put(
    &self,
    key_expr: String,
    payload: Either<String, Uint8Array>,
    options: Option<PutOptions>,
  ) -> Result<()> {
    let mut builder = self.inner.put(key_expr, to_zbytes(payload));
    if let Some(options) = options {
      if let Some(encoding) = options.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(attachment) = options.attachment {
        builder = builder.attachment(to_zbytes(attachment));
      }
      if let Some(congestion_control) = options.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = options.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = options.express {
        builder = builder.express(express);
      }
      if let Some(reliability) = options.reliability {
        builder = builder.reliability(reliability.into());
      }
      if let Some(allowed_destination) = options.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timestamp) = options.timestamp {
        builder = builder.timestamp(timestamp.to_zenoh()?);
      }
      if let Some(source_info) = options.source_info {
        builder = builder.source_info(source_info.to_zenoh()?);
      }
    }
    builder.await.map_err(to_napi_err)
  }

  /// Publish a `Delete` sample for `key_expr`, signalling that the value at that
  /// key is no longer valid.
  #[napi]
  pub async fn delete(&self, key_expr: String, options: Option<DeleteOptions>) -> Result<()> {
    let mut builder = self.inner.delete(key_expr);
    if let Some(options) = options {
      if let Some(attachment) = options.attachment {
        builder = builder.attachment(to_zbytes(attachment));
      }
      if let Some(congestion_control) = options.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = options.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = options.express {
        builder = builder.express(express);
      }
      if let Some(reliability) = options.reliability {
        builder = builder.reliability(reliability.into());
      }
      if let Some(allowed_destination) = options.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timestamp) = options.timestamp {
        builder = builder.timestamp(timestamp.to_zenoh()?);
      }
      if let Some(source_info) = options.source_info {
        builder = builder.source_info(source_info.to_zenoh()?);
      }
    }
    builder.await.map_err(to_napi_err)
  }

  /// Query `selector` and receive the matching queryables' replies through a
  /// channel, consumable as an async iterator or via `recv`/`tryRecv`.
  #[napi]
  pub async fn get(&self, selector: String, options: Option<GetOptions>) -> Result<Replies> {
    let mut builder = self.inner.get(selector);
    let mut channel = None;
    if let Some(options) = options {
      if let Some(payload) = options.payload {
        builder = builder.payload(to_zbytes(payload));
      }
      if let Some(encoding) = options.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(attachment) = options.attachment {
        builder = builder.attachment(to_zbytes(attachment));
      }
      if let Some(congestion_control) = options.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = options.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = options.express {
        builder = builder.express(express);
      }
      if let Some(target) = options.target {
        builder = builder.target(target.into());
      }
      if let Some(consolidation) = options.consolidation {
        builder = builder.consolidation(zenoh::query::ConsolidationMode::from(consolidation));
      }
      if let Some(allowed_destination) = options.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timeout) = options.timeout {
        builder = builder.timeout(std::time::Duration::from_millis(timeout.into()));
      }
      if let Some(accept_replies) = options.accept_replies {
        builder = builder.accept_replies(accept_replies.into());
      }
      if let Some(source_info) = options.source_info {
        builder = builder.source_info(source_info.to_zenoh()?);
      }
      channel = options.handler;
    }
    Replies::from_session_get(builder, channel).await
  }

  /// Declare a [`Publisher`] for `key_expr`. Its QoS is fixed at declaration
  /// time; per-publication `put`/`delete` can override only payload fields.
  #[napi]
  pub async fn declare_publisher(
    &self,
    key_expr: String,
    options: Option<PublisherOptions>,
  ) -> Result<Publisher> {
    let mut builder = self.inner.declare_publisher(key_expr);
    if let Some(options) = options {
      if let Some(encoding) = options.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(congestion_control) = options.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = options.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = options.express {
        builder = builder.express(express);
      }
      if let Some(reliability) = options.reliability {
        builder = builder.reliability(reliability.into());
      }
      if let Some(allowed_destination) = options.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
    }
    let publisher = builder.await.map_err(to_napi_err)?;
    Ok(Publisher::new(publisher))
  }

  /// Declare a [`Subscriber`] for `key_expr`. Samples are delivered through a
  /// FIFO channel, consumable as an async iterator or via `recv`/`tryRecv`.
  #[napi]
  pub async fn declare_subscriber(
    &self,
    key_expr: String,
    options: Option<SubscriberOptions>,
  ) -> Result<Subscriber> {
    let mut builder = self.inner.declare_subscriber(key_expr);
    let mut channel = None;
    if let Some(options) = options {
      if let Some(allowed_origin) = options.allowed_origin {
        builder = builder.allowed_origin(allowed_origin.into());
      }
      channel = options.handler;
    }
    Subscriber::declare(builder, channel).await
  }

  /// Declare a [`Queryable`] for `key_expr`. Queries are delivered through a
  /// channel, consumable as an async iterator or via `recv`/`tryRecv`.
  #[napi]
  pub async fn declare_queryable(
    &self,
    key_expr: String,
    options: Option<QueryableOptions>,
  ) -> Result<Queryable> {
    let mut builder = self.inner.declare_queryable(key_expr);
    let mut channel = None;
    if let Some(options) = options {
      if let Some(complete) = options.complete {
        builder = builder.complete(complete);
      }
      if let Some(allowed_origin) = options.allowed_origin {
        builder = builder.allowed_origin(allowed_origin.into());
      }
      channel = options.handler;
    }
    Queryable::declare(builder, channel).await
  }

  /// Declare a [`Querier`] for `key_expr` — a reusable handle for querying that
  /// key, with query settings fixed here at declaration time.
  #[napi]
  pub async fn declare_querier(
    &self,
    key_expr: String,
    options: Option<QuerierOptions>,
  ) -> Result<Querier> {
    let mut builder = self.inner.declare_querier(key_expr);
    if let Some(options) = options {
      if let Some(congestion_control) = options.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = options.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = options.express {
        builder = builder.express(express);
      }
      if let Some(target) = options.target {
        builder = builder.target(target.into());
      }
      if let Some(consolidation) = options.consolidation {
        builder = builder.consolidation(zenoh::query::ConsolidationMode::from(consolidation));
      }
      if let Some(allowed_destination) = options.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timeout) = options.timeout {
        builder = builder.timeout(std::time::Duration::from_millis(timeout.into()));
      }
      if let Some(accept_replies) = options.accept_replies {
        builder = builder.accept_replies(accept_replies.into());
      }
    }
    let querier = builder.await.map_err(to_napi_err)?;
    Ok(Querier::new(querier))
  }

  /// Create a new timestamp using this session's hybrid logical clock.
  #[napi]
  pub fn new_timestamp(&self) -> Timestamp {
    Timestamp::from_zenoh(&self.inner.new_timestamp())
  }

  /// Access information about this session and the nodes it is connected to.
  #[napi]
  pub fn info(&self) -> SessionInfo {
    SessionInfo {
      inner: self.inner.clone(),
    }
  }

  /// The Zenoh ID of this session, as a hex string.
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  /// Whether the session has been closed.
  #[napi(getter)]
  pub fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }
}

/// Information about a [`Session`] and the routers/peers it is connected to.
#[napi]
pub struct SessionInfo {
  inner: zenoh::Session,
}

#[napi]
impl SessionInfo {
  /// The Zenoh ID of this session.
  #[napi]
  pub async fn zid(&self) -> String {
    self.inner.info().zid().await.to_string()
  }

  /// The Zenoh IDs of the routers this session is currently connected to.
  #[napi]
  pub async fn routers_zid(&self) -> Vec<String> {
    self
      .inner
      .info()
      .routers_zid()
      .await
      .map(|zid| zid.to_string())
      .collect()
  }

  /// The Zenoh IDs of the peers this session is currently connected to.
  #[napi]
  pub async fn peers_zid(&self) -> Vec<String> {
    self
      .inner
      .info()
      .peers_zid()
      .await
      .map(|zid| zid.to_string())
      .collect()
  }
}
