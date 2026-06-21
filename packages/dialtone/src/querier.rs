use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::to_napi_err;
use crate::handlers::ChannelHandler;
use crate::keyexpr::KeyExpr;
use crate::macros::apply_options;
use crate::matching::{MatchingListener, MatchingStatus};
use crate::qos::{CongestionControl, Priority};
use crate::query::{ConsolidationMode, QueryTarget, Replies, ReplyKeyExpr};
use crate::sample::{Locality, SourceInfo};
use crate::session::EntityGlobalId;

/// Options for [`Session::declareQuerier`]. These settings are fixed for the
/// querier's lifetime; per-`get` only the payload and parameters may vary.
#[napi(object)]
pub struct QuerierOptions {
  /// Congestion control strategy for queries (default: `Block`).
  pub congestion_control: Option<CongestionControl>,
  /// Priority of queries (default: `Data`).
  pub priority: Option<Priority>,
  /// When `true`, queries are sent unbatched, trading throughput for latency.
  pub express: Option<bool>,
  /// Which queryables should answer (default: `BestMatching`).
  pub target: Option<QueryTarget>,
  /// How replies are consolidated before delivery (default: `Auto`).
  pub consolidation: Option<ConsolidationMode>,
  /// Restrict which queryables receive queries (default: `Any`).
  pub allowed_destination: Option<Locality>,
  /// How long to wait for replies, in milliseconds.
  pub timeout: Option<u32>,
  /// Which reply key expressions to accept (default: `MatchingQuery`).
  pub accept_replies: Option<ReplyKeyExpr>,
}

/// Options for [`Querier::get`].
#[napi(object)]
pub struct QuerierGetOptions {
  /// Selector parameters (the part after `?`) for this query.
  pub parameters: Option<String>,
  /// Payload to send alongside the query.
  pub payload: Option<Either<String, Uint8Array>>,
  /// Encoding of the query payload.
  pub encoding: Option<String>,
  /// Optional attachment carried alongside the query.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// Source metadata (producing entity + sequence number).
  pub source_info: Option<SourceInfo>,
  /// Channel handler (FIFO or Ring) backing reply delivery. Defaults to FIFO.
  pub handler: Option<ChannelHandler>,
}

/// A querier bound to a key expression, with query settings fixed at
/// declaration time — the query analog of a [`Publisher`]. Create one with
/// [`Session::declareQuerier`], then issue queries with `get`.
#[napi]
pub struct Querier {
  inner: Option<zenoh::query::Querier<'static>>,
}

impl Querier {
  pub(crate) fn new(inner: zenoh::query::Querier<'static>) -> Self {
    Self { inner: Some(inner) }
  }

  fn querier(&self) -> Result<&zenoh::query::Querier<'static>> {
    self
      .inner
      .as_ref()
      .ok_or_else(|| Error::from_reason("querier has been undeclared"))
  }
}

#[napi]
impl Querier {
  /// Issue a query on this querier's key expression and receive the matching
  /// queryables' replies through a channel, consumable as an async iterator or
  /// via `recv`/`tryRecv`.
  #[napi]
  pub async fn get(&self, options: Option<QuerierGetOptions>) -> Result<Replies> {
    let mut builder = self.querier()?.get();
    let mut channel = None;
    if let Some(options) = options {
      apply_options!(builder, options, {
        parameters,
        payload => zbytes,
        encoding,
        attachment => zbytes,
        source_info => try_zenoh,
      });
      channel = options.handler;
    }
    Replies::from_querier_get(builder, channel).await
  }

  /// Whether any queryables currently match this querier's key expression.
  #[napi]
  pub async fn matching_status(&self) -> Result<MatchingStatus> {
    let querier = self.querier()?;
    let status = querier.matching_status().await.map_err(to_napi_err)?;
    Ok(MatchingStatus {
      matching: status.matching(),
    })
  }

  /// Declare a [`MatchingListener`] that notifies when this querier's set of
  /// matching queryables changes. The optional channel `handler` (FIFO or Ring)
  /// backs the notifications; defaults to FIFO.
  #[napi]
  pub async fn matching_listener(
    &self,
    handler: Option<ChannelHandler>,
  ) -> Result<MatchingListener> {
    let builder = self.querier()?.matching_listener();
    MatchingListener::declare(builder, handler).await
  }

  /// Undeclare this querier. Subsequent operations on it will error.
  ///
  /// Resolves synchronously, so awaiting the returned value is optional.
  #[napi]
  pub fn undeclare(&mut self) -> Result<()> {
    use zenoh::Wait;
    match self.inner.take() {
      Some(querier) => querier.undeclare().wait().map_err(to_napi_err),
      None => Ok(()),
    }
  }

  /// The key expression this querier sends queries on.
  #[napi(getter)]
  pub fn key_expr(&self) -> Result<KeyExpr> {
    Ok(KeyExpr::from_zenoh(
      self.querier()?.key_expr().clone().into_owned(),
    ))
  }

  /// The congestion control strategy applied to queries.
  #[napi(getter)]
  pub fn congestion_control(&self) -> Result<CongestionControl> {
    Ok(self.querier()?.congestion_control().into())
  }

  /// The priority of queries.
  #[napi(getter)]
  pub fn priority(&self) -> Result<Priority> {
    Ok(self.querier()?.priority().into())
  }

  /// Which reply key expressions this querier accepts.
  #[napi(getter)]
  pub fn accept_replies(&self) -> Result<ReplyKeyExpr> {
    Ok(self.querier()?.accept_replies().into())
  }

  /// This querier's globally-unique entity id.
  #[napi(getter)]
  pub fn id(&self) -> Result<EntityGlobalId> {
    Ok(EntityGlobalId::from_zenoh(self.querier()?.id()))
  }
}
