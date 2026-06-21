use std::future::Future;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::handlers::{DefaultHandler, FifoChannelHandler, RingChannelHandler};
use zenoh::query::QueryableBuilder;

use crate::bytes::to_zbytes;
use crate::error::to_napi_err;
use crate::handlers::{self, ChannelHandler, ChannelReceiver, ChannelType};
use crate::keyexpr::{KeyExpr, KeyExprArg};
use crate::macros::apply_options;
use crate::qos::{CongestionControl, Priority};
use crate::query::ReplyKeyExpr;
use crate::sample::{Locality, SourceInfo};
use crate::session::EntityGlobalId;
use crate::time::Timestamp;

/// Options for [`Query::reply`].
#[napi(object)]
pub struct ReplyOptions {
  /// Encoding of the reply payload.
  pub encoding: Option<String>,
  /// Optional attachment carried alongside the reply.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// When `true`, the reply is sent unbatched, trading throughput for latency.
  pub express: Option<bool>,
  /// Timestamp to attach; obtain one from [`Session::newTimestamp`].
  pub timestamp: Option<Timestamp>,
  /// Source metadata (producing entity + sequence number).
  pub source_info: Option<SourceInfo>,
}

/// Options for [`Query::replyErr`].
#[napi(object)]
pub struct ReplyErrOptions {
  /// Encoding of the error payload.
  pub encoding: Option<String>,
}

/// Options for [`Query::replyDel`].
#[napi(object)]
pub struct ReplyDelOptions {
  /// Optional attachment carried alongside the deletion.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// When `true`, the deletion is sent unbatched, trading throughput for latency.
  pub express: Option<bool>,
  /// Timestamp to attach; obtain one from [`Session::newTimestamp`].
  pub timestamp: Option<Timestamp>,
  /// Source metadata (producing entity + sequence number).
  pub source_info: Option<SourceInfo>,
}

/// A query received by a [`Queryable`], to be answered with `reply` / `replyErr`
/// / `replyDel` (any number of times, including none).
///
/// The query is finalized when this object is dropped, so keep it alive until
/// you have sent every reply you intend to.
#[napi]
pub struct Query {
  inner: zenoh::query::Query,
}

impl Query {
  pub(crate) fn new(inner: zenoh::query::Query) -> Self {
    Self { inner }
  }
}

#[napi]
impl Query {
  /// The full selector (key expression plus parameters) this query targets.
  #[napi(getter)]
  pub fn selector(&self) -> String {
    self.inner.selector().to_string()
  }

  /// The key expression this query targets.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_zenoh(self.inner.key_expr().clone().into_owned())
  }

  /// The selector parameters (the part after `?`), as a raw string.
  #[napi(getter)]
  pub fn parameters(&self) -> String {
    self.inner.parameters().as_str().to_string()
  }

  /// The query's payload bytes, if it carried any.
  #[napi(getter)]
  pub fn payload(&self) -> Option<Buffer> {
    self
      .inner
      .payload()
      .map(|payload| Buffer::from(payload.to_bytes().to_vec()))
  }

  /// The payload encoding, if the query carried a payload.
  #[napi(getter)]
  pub fn encoding(&self) -> Option<String> {
    self.inner.encoding().map(|encoding| encoding.to_string())
  }

  /// The query's attachment bytes, if any.
  #[napi(getter)]
  pub fn attachment(&self) -> Option<Buffer> {
    self
      .inner
      .attachment()
      .map(|attachment| Buffer::from(attachment.to_bytes().to_vec()))
  }

  /// The query's source metadata, if any.
  #[napi(getter)]
  pub fn source_info(&self) -> Option<SourceInfo> {
    self.inner.source_info().map(SourceInfo::from_zenoh)
  }

  /// Which reply key expressions this query accepts.
  #[napi(getter)]
  pub fn accepts_replies(&self) -> ReplyKeyExpr {
    self.inner.accepts_replies().into()
  }

  /// The priority the query was sent with.
  #[napi(getter)]
  pub fn priority(&self) -> Priority {
    self.inner.priority().into()
  }

  /// The congestion control the query was sent with.
  #[napi(getter)]
  pub fn congestion_control(&self) -> CongestionControl {
    self.inner.congestion_control().into()
  }

  /// Whether the query was sent express (unbatched).
  #[napi(getter)]
  pub fn express(&self) -> bool {
    self.inner.express()
  }

  /// Reply to this query with a `Put` sample for `key_expr`.
  #[napi]
  pub async fn reply(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    payload: Either<String, Uint8Array>,
    options: Option<ReplyOptions>,
  ) -> Result<()> {
    let mut builder = self.inner.reply(key_expr.0, to_zbytes(payload));
    if let Some(options) = options {
      apply_options!(builder, options, {
        encoding,
        attachment => zbytes,
        express,
        timestamp => try_zenoh,
        source_info => try_zenoh,
      });
    }
    builder.await.map_err(to_napi_err)
  }

  /// Reply to this query with an error response.
  #[napi]
  pub async fn reply_err(
    &self,
    payload: Either<String, Uint8Array>,
    options: Option<ReplyErrOptions>,
  ) -> Result<()> {
    let mut builder = self.inner.reply_err(to_zbytes(payload));
    if let Some(options) = options {
      apply_options!(builder, options, {
        encoding,
      });
    }
    builder.await.map_err(to_napi_err)
  }

  /// Reply to this query with a `Delete` sample for `key_expr`.
  #[napi]
  pub async fn reply_del(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<ReplyDelOptions>,
  ) -> Result<()> {
    let mut builder = self.inner.reply_del(key_expr.0);
    if let Some(options) = options {
      apply_options!(builder, options, {
        attachment => zbytes,
        express,
        timestamp => try_zenoh,
        source_info => try_zenoh,
      });
    }
    builder.await.map_err(to_napi_err)
  }
}

type FifoQueryable = zenoh::query::Queryable<Arc<FifoChannelHandler<zenoh::query::Query>>>;
type RingQueryable = zenoh::query::Queryable<Arc<RingChannelHandler<zenoh::query::Query>>>;

/// The declared queryable, kept alive (and undeclarable) regardless of which
/// channel kind backs it.
enum QueryableInner {
  Fifo(FifoQueryable),
  Ring(RingQueryable),
}

impl QueryableInner {
  fn key_expr(&self) -> zenoh::key_expr::KeyExpr<'static> {
    match self {
      QueryableInner::Fifo(queryable) => queryable.key_expr().clone().into_owned(),
      QueryableInner::Ring(queryable) => queryable.key_expr().clone().into_owned(),
    }
  }

  fn id(&self) -> zenoh::session::EntityGlobalId {
    match self {
      QueryableInner::Fifo(queryable) => queryable.id(),
      QueryableInner::Ring(queryable) => queryable.id(),
    }
  }

  fn undeclare(self) -> Result<()> {
    use zenoh::Wait;
    match self {
      QueryableInner::Fifo(queryable) => queryable.undeclare().wait().map_err(to_napi_err),
      QueryableInner::Ring(queryable) => queryable.undeclare().wait().map_err(to_napi_err),
    }
  }
}

/// Options for [`Session::declareQueryable`].
#[napi(object)]
pub struct QueryableOptions {
  /// Whether this queryable can answer the full queried key expression on its
  /// own; lets queriers using `AllComplete` targeting reach it (default: false).
  pub complete: Option<bool>,
  /// Restrict which queriers' queries are accepted (default: `Any`).
  pub allowed_origin: Option<Locality>,
  /// Channel handler (FIFO or Ring) backing delivery. Defaults to FIFO.
  pub handler: Option<ChannelHandler>,
}

/// A queryable that delivers [`Query`]s through a channel.
///
/// Consume it with `for await (const query of queryable)`, or pull queries
/// individually with `recv()` / `tryRecv()`. Iteration ends (yields `null`)
/// once the queryable is undeclared — its buffered queries are dropped with the
/// handler, as in zenoh — or once the session/link closes and any buffered
/// queries have been drained.
#[napi(async_iterator)]
pub struct Queryable {
  inner: Option<QueryableInner>,
  /// Released together with `inner` on undeclare, so the handler (and any
  /// queries still buffered in it) is dropped exactly as zenoh's own `undeclare`
  /// does, rather than left draining after the queryable is gone.
  receiver: Option<ChannelReceiver<zenoh::query::Query>>,
}

impl Queryable {
  pub(crate) async fn declare(
    builder: QueryableBuilder<'_, '_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let (inner, receiver) = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<zenoh::query::Query>(capacity);
        let queryable = builder.with(handler).await.map_err(to_napi_err)?;
        (QueryableInner::Fifo(queryable), receiver)
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<zenoh::query::Query>(capacity);
        let queryable = builder.with(handler).await.map_err(to_napi_err)?;
        (QueryableInner::Ring(queryable), receiver)
      }
    };
    Ok(Self {
      inner: Some(inner),
      receiver: Some(receiver),
    })
  }

  fn get(&self) -> Result<&QueryableInner> {
    self
      .inner
      .as_ref()
      .ok_or_else(|| Error::from_reason("queryable has been undeclared"))
  }
}

#[napi]
impl Queryable {
  /// Wait for the next query, resolving to `null` once the queryable is
  /// undeclared, or once it closes and all buffered queries have been drained.
  #[napi]
  pub async fn recv(&self) -> Result<Option<Query>> {
    let receiver = self.receiver.clone();
    match receiver {
      Some(receiver) => Ok(receiver.recv().await.map(Query::new)),
      None => Ok(None),
    }
  }

  /// Return a buffered query if one is immediately available, or `null` if the
  /// channel is currently empty. Throws once the queryable has disconnected
  /// (undeclared, or the session closed and all buffered queries drained),
  /// letting a polling loop tell "nothing yet" apart from "closed".
  #[napi]
  pub fn try_recv(&self) -> Result<Option<Query>> {
    match &self.receiver {
      Some(receiver) => receiver
        .try_recv()
        .map(|query| query.map(Query::new))
        .map_err(to_napi_err),
      None => Err(Error::from_reason("queryable has been undeclared")),
    }
  }

  /// Undeclare the queryable. Iteration / `recv` then end and `tryRecv` throws;
  /// any buffered queries are dropped with the handler. Resolves synchronously.
  #[napi]
  pub fn undeclare(&mut self) -> Result<()> {
    // Release the receiver with the declaration: zenoh drops the handler (and
    // anything still buffered in it) as part of undeclaring, so mirror that
    // instead of leaving a FIFO buffer draining after the queryable is gone.
    self.receiver = None;
    match self.inner.take() {
      Some(inner) => inner.undeclare(),
      None => Ok(()),
    }
  }

  /// The key expression this queryable answers.
  #[napi(getter)]
  pub fn key_expr(&self) -> Result<KeyExpr> {
    Ok(KeyExpr::from_zenoh(self.get()?.key_expr()))
  }

  /// This queryable's globally-unique entity id.
  #[napi(getter)]
  pub fn id(&self) -> Result<EntityGlobalId> {
    Ok(EntityGlobalId::from_zenoh(self.get()?.id()))
  }
}

#[napi]
impl AsyncGenerator for Queryable {
  type Yield = Query;
  type Next = ();
  type Return = ();

  fn next(
    &mut self,
    _value: Option<Self::Next>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let receiver = self.receiver.clone();
    async move {
      match receiver {
        Some(receiver) => Ok(receiver.recv().await.map(Query::new)),
        None => Ok(None),
      }
    }
  }
}
