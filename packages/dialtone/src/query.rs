use std::future::Future;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::handlers::DefaultHandler;
use zenoh::liveliness::LivelinessGetBuilder;
use zenoh::query::QuerierGetBuilder;
use zenoh::session::SessionGetBuilder;

use crate::error::to_napi_err;
use crate::handlers::{self, ChannelHandler, ChannelReceiver, ChannelType};
use crate::qos::{CongestionControl, Priority};
use crate::sample::{Locality, Sample, SourceInfo};
use crate::session::EntityGlobalId;

/// How a query selects which queryables answer it.
#[napi(string_enum)]
pub enum QueryTarget {
  /// Route to the best-matching queryable Zenoh can find (default).
  BestMatching,
  /// Deliver to every queryable matching the query's key expression.
  All,
  /// Deliver to every matching queryable declared as `complete`.
  AllComplete,
}

impl From<QueryTarget> for zenoh::query::QueryTarget {
  fn from(value: QueryTarget) -> Self {
    match value {
      QueryTarget::BestMatching => zenoh::query::QueryTarget::BestMatching,
      QueryTarget::All => zenoh::query::QueryTarget::All,
      QueryTarget::AllComplete => zenoh::query::QueryTarget::AllComplete,
    }
  }
}

/// How replies to a query are consolidated before being delivered.
#[napi(string_enum)]
pub enum ConsolidationMode {
  /// Consolidate automatically, based on the query (default).
  Auto,
  /// No consolidation: duplicate samples for the same key may be delivered.
  None,
  /// Forward samples immediately, dropping any with an older-or-equal timestamp
  /// already seen for the same key.
  Monotonic,
  /// Hold replies back and deliver only the latest-timestamped sample per key.
  Latest,
}

impl From<ConsolidationMode> for zenoh::query::ConsolidationMode {
  fn from(value: ConsolidationMode) -> Self {
    match value {
      ConsolidationMode::Auto => zenoh::query::ConsolidationMode::Auto,
      ConsolidationMode::None => zenoh::query::ConsolidationMode::None,
      ConsolidationMode::Monotonic => zenoh::query::ConsolidationMode::Monotonic,
      ConsolidationMode::Latest => zenoh::query::ConsolidationMode::Latest,
    }
  }
}

/// Which reply key expressions a query is willing to accept.
#[napi(string_enum)]
pub enum ReplyKeyExpr {
  /// Accept replies whose key expression need not match the query's.
  Any,
  /// Accept only replies whose key expression matches the query's (default).
  MatchingQuery,
}

impl From<ReplyKeyExpr> for zenoh::query::ReplyKeyExpr {
  fn from(value: ReplyKeyExpr) -> Self {
    match value {
      ReplyKeyExpr::Any => zenoh::query::ReplyKeyExpr::Any,
      ReplyKeyExpr::MatchingQuery => zenoh::query::ReplyKeyExpr::MatchingQuery,
    }
  }
}

impl From<zenoh::query::ReplyKeyExpr> for ReplyKeyExpr {
  fn from(value: zenoh::query::ReplyKeyExpr) -> Self {
    match value {
      zenoh::query::ReplyKeyExpr::Any => ReplyKeyExpr::Any,
      zenoh::query::ReplyKeyExpr::MatchingQuery => ReplyKeyExpr::MatchingQuery,
    }
  }
}

/// A query reply carrying a [`Sample`] — the value a queryable returned.
///
/// One arm of the reply union returned by `get`: `sample` is the queried value.
/// Pairs with [`ReplyError`]; discriminate with `if (reply.sample)`.
#[napi]
pub struct ReplySample {
  sample: zenoh::sample::Sample,
  replier_id: Option<EntityGlobalId>,
}

impl ReplySample {
  pub(crate) fn new(sample: zenoh::sample::Sample, replier_id: Option<EntityGlobalId>) -> Self {
    Self { sample, replier_id }
  }
}

#[napi]
impl ReplySample {
  /// The sample this reply carries.
  #[napi(getter)]
  pub fn sample(&self) -> Sample {
    Sample::new(self.sample.clone())
  }

  /// The id of the entity that produced this reply, if known.
  #[napi(getter)]
  pub fn replier_id(&self) -> Option<EntityGlobalId> {
    self.replier_id.clone()
  }
}

/// A query reply carrying an error response instead of a sample — mirrors
/// zenoh's `ReplyError` (its `payload` / `encoding`).
///
/// The other arm of the reply union returned by `get`: `sample` is `null`, so
/// `if (reply.sample)` discriminates it from [`ReplySample`].
#[napi]
pub struct ReplyError {
  error: zenoh::query::ReplyError,
  replier_id: Option<EntityGlobalId>,
}

impl ReplyError {
  pub(crate) fn new(error: zenoh::query::ReplyError, replier_id: Option<EntityGlobalId>) -> Self {
    Self { error, replier_id }
  }
}

#[napi]
impl ReplyError {
  /// Always `null` on an error reply — the discriminant against [`ReplySample`].
  #[napi(getter)]
  pub fn sample(&self) -> Null {
    Null
  }

  /// The error payload bytes.
  #[napi(getter)]
  pub fn payload(&self) -> Buffer {
    Buffer::from(self.error.payload().to_bytes().to_vec())
  }

  /// The error payload encoding.
  #[napi(getter)]
  pub fn encoding(&self) -> String {
    self.error.encoding().to_string()
  }

  /// The id of the entity that produced this reply, if known.
  #[napi(getter)]
  pub fn replier_id(&self) -> Option<EntityGlobalId> {
    self.replier_id.clone()
  }
}

/// Options for [`Session::get`].
#[napi(object)]
pub struct GetOptions {
  /// Payload to send alongside the query.
  pub payload: Option<Either<String, Uint8Array>>,
  /// Encoding of the query payload.
  pub encoding: Option<String>,
  /// Optional attachment carried alongside the query.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// Congestion control strategy for the query (default: `Block`).
  pub congestion_control: Option<CongestionControl>,
  /// Priority of the query (default: `Data`).
  pub priority: Option<Priority>,
  /// When `true`, the query is sent unbatched, trading throughput for latency.
  pub express: Option<bool>,
  /// Which queryables should answer (default: `BestMatching`).
  pub target: Option<QueryTarget>,
  /// How replies are consolidated before delivery (default: `Auto`).
  pub consolidation: Option<ConsolidationMode>,
  /// Restrict which queryables receive the query (default: `Any`).
  pub allowed_destination: Option<Locality>,
  /// How long to wait for replies, in milliseconds.
  pub timeout: Option<u32>,
  /// Which reply key expressions to accept (default: `MatchingQuery`).
  pub accept_replies: Option<ReplyKeyExpr>,
  /// Source metadata (producing entity + sequence number).
  pub source_info: Option<SourceInfo>,
  /// Channel handler (FIFO or Ring) backing reply delivery. Defaults to FIFO.
  pub handler: Option<ChannelHandler>,
}

/// Convert a zenoh reply into the discriminated union arm exposed to JS.
fn reply_to_arms(reply: zenoh::query::Reply) -> Either<ReplySample, ReplyError> {
  let replier_id = reply.replier_id().map(EntityGlobalId::from_zenoh);
  match reply.into_result() {
    Ok(sample) => Either::A(ReplySample::new(sample, replier_id)),
    Err(error) => Either::B(ReplyError::new(error, replier_id)),
  }
}

/// The replies to a [`Session::get`], delivered through a channel.
///
/// Consume it with `for await (const reply of replies)`, or pull replies with
/// `recv()` / `tryRecv()`. Each reply is a [`ReplySample`] or a [`ReplyError`] —
/// discriminate with `if (reply.sample)`. Iteration ends (yields `null`) once
/// every queryable has answered or the query times out. A query is not a
/// declared entity, so there is nothing to undeclare.
#[napi(async_iterator)]
pub struct Replies {
  receiver: ChannelReceiver<zenoh::query::Reply>,
}

impl Replies {
  pub(crate) async fn from_session_get(
    builder: SessionGetBuilder<'_, '_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let receiver = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<zenoh::query::Reply>(capacity);
        builder.with(handler).await.map_err(to_napi_err)?;
        receiver
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<zenoh::query::Reply>(capacity);
        builder.with(handler).await.map_err(to_napi_err)?;
        receiver
      }
    };
    Ok(Self { receiver })
  }

  pub(crate) async fn from_liveliness_get(
    builder: LivelinessGetBuilder<'_, '_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let receiver = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<zenoh::query::Reply>(capacity);
        builder.with(handler).await.map_err(to_napi_err)?;
        receiver
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<zenoh::query::Reply>(capacity);
        builder.with(handler).await.map_err(to_napi_err)?;
        receiver
      }
    };
    Ok(Self { receiver })
  }

  pub(crate) async fn from_querier_get(
    builder: QuerierGetBuilder<'_, '_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let receiver = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<zenoh::query::Reply>(capacity);
        builder.with(handler).await.map_err(to_napi_err)?;
        receiver
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<zenoh::query::Reply>(capacity);
        builder.with(handler).await.map_err(to_napi_err)?;
        receiver
      }
    };
    Ok(Self { receiver })
  }
}

#[napi]
impl Replies {
  /// Wait for the next reply, resolving to `null` once the query is complete
  /// and all buffered replies have been drained.
  #[napi]
  pub async fn recv(&self) -> Result<Option<Either<ReplySample, ReplyError>>> {
    Ok(self.receiver.recv().await.map(reply_to_arms))
  }

  /// Return a buffered reply if one is immediately available, or `null` if none
  /// is ready yet. Throws once the query is complete and all replies drained,
  /// letting a polling loop tell "nothing yet" apart from "done".
  #[napi]
  pub fn try_recv(&self) -> Result<Option<Either<ReplySample, ReplyError>>> {
    self
      .receiver
      .try_recv()
      .map(|reply| reply.map(reply_to_arms))
      .map_err(to_napi_err)
  }
}

#[napi]
impl AsyncGenerator for Replies {
  type Yield = Either<ReplySample, ReplyError>;
  type Next = ();
  type Return = ();

  fn next(
    &mut self,
    _value: Option<Self::Next>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let receiver = self.receiver.clone();
    async move { Ok(receiver.recv().await.map(reply_to_arms)) }
  }
}
