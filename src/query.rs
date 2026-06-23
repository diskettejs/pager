use napi::bindgen_prelude::{Either, Uint8Array};
use napi_derive::napi;
use zenoh::bytes::ZBytes;
use zenoh::query::{
  ConsolidationMode as ZConsolidationMode, Query as ZQuery, QueryTarget as ZQueryTarget,
  ReplyKeyExpr as ZReplyKeyExpr,
};

use crate::bytes::Bytes;
use crate::encoding::Encoding;
use crate::keyexpr::{KeyExpr, KeyExprArg};
use crate::options::{ReplyDelOptions, ReplyErrOptions, ReplyOptions};
use crate::parameters::Parameters;
use crate::qos::{CongestionControl, Priority};
use crate::selector::Selector;
use crate::source_info::SourceInfo;

/// The kind of queryables that should be targeted by a query.
#[napi(string_enum)]
pub enum QueryTarget {
  /// Let zenoh find the best matching queryable capable of serving the query.
  BestMatching,
  /// Deliver the query to all matching queryables.
  All,
  /// Deliver the query to all matching queryables declared as complete.
  AllComplete,
}

impl From<QueryTarget> for ZQueryTarget {
  fn from(value: QueryTarget) -> Self {
    match value {
      QueryTarget::BestMatching => Self::BestMatching,
      QueryTarget::All => Self::All,
      QueryTarget::AllComplete => Self::AllComplete,
    }
  }
}

impl From<ZQueryTarget> for QueryTarget {
  fn from(value: ZQueryTarget) -> Self {
    match value {
      ZQueryTarget::BestMatching => Self::BestMatching,
      ZQueryTarget::All => Self::All,
      ZQueryTarget::AllComplete => Self::AllComplete,
    }
  }
}

/// How replies to a query are consolidated before being delivered.
#[napi(string_enum)]
pub enum ConsolidationMode {
  /// Automatic consolidation based on the queryable's preferences.
  Auto,
  /// No consolidation: multiple samples may be received for the same key.
  None,
  /// Forward samples immediately, dropping ones superseded by a newer timestamp.
  Monotonic,
  /// Hold back to send only the samples with the highest timestamp per key.
  Latest,
}

impl From<ConsolidationMode> for ZConsolidationMode {
  fn from(value: ConsolidationMode) -> Self {
    match value {
      ConsolidationMode::Auto => Self::Auto,
      ConsolidationMode::None => Self::None,
      ConsolidationMode::Monotonic => Self::Monotonic,
      ConsolidationMode::Latest => Self::Latest,
    }
  }
}

impl From<ZConsolidationMode> for ConsolidationMode {
  fn from(value: ZConsolidationMode) -> Self {
    match value {
      ZConsolidationMode::Auto => Self::Auto,
      ZConsolidationMode::None => Self::None,
      ZConsolidationMode::Monotonic => Self::Monotonic,
      ZConsolidationMode::Latest => Self::Latest,
    }
  }
}

/// Whether replies whose key expression doesn't match the query are accepted.
#[napi(string_enum)]
pub enum ReplyKeyExpr {
  /// Accept replies whose key expressions may not match the query.
  Any,
  /// Accept only replies whose key expressions match the query.
  MatchingQuery,
}

impl From<ReplyKeyExpr> for ZReplyKeyExpr {
  fn from(value: ReplyKeyExpr) -> Self {
    match value {
      ReplyKeyExpr::Any => Self::Any,
      ReplyKeyExpr::MatchingQuery => Self::MatchingQuery,
    }
  }
}

impl From<ZReplyKeyExpr> for ReplyKeyExpr {
  fn from(value: ZReplyKeyExpr) -> Self {
    match value {
      ZReplyKeyExpr::Any => Self::Any,
      ZReplyKeyExpr::MatchingQuery => Self::MatchingQuery,
    }
  }
}

/// A query received by a `Queryable` — a request this session is expected to
/// answer by sending zero or more replies.
///
/// Reply with `reply` (a `Put` sample), `replyErr` (an error), or `replyDel` (a
/// `Delete` sample). A query may be answered any number of times. When the
/// `Query` is dropped without further replies, zenoh finalizes it; nothing here
/// needs to be called to "close" it.
#[napi]
pub struct Query {
  inner: ZQuery,
}

impl Query {
  /// Internal constructor: wrap the owned `zenoh` query delivered by the handler.
  pub(crate) fn from_inner(inner: ZQuery) -> Self {
    Query { inner }
  }
}

#[napi]
impl Query {
  /// The full selector (key expression + parameters) of this query.
  #[napi(getter)]
  pub fn selector(&self) -> Selector {
    Selector::from_inner(self.inner.selector().into_owned())
  }

  /// The key expression part of this query's selector.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.inner.key_expr().clone())
  }

  /// The parameters part of this query's selector.
  #[napi(getter)]
  pub fn parameters(&self) -> Parameters {
    Parameters::from_inner(self.inner.parameters().clone())
  }

  /// This query's payload, or `null` if it carries none.
  #[napi(getter)]
  pub fn payload(&self) -> Option<Bytes> {
    self.inner.payload().cloned().map(Bytes::from_inner)
  }

  /// The encoding of this query's payload, or `null` if it carries no payload.
  #[napi(getter)]
  pub fn encoding(&self) -> Option<Encoding> {
    self.inner.encoding().cloned().map(Encoding::from_inner)
  }

  /// This query's attachment, or `null` if it carries none.
  #[napi(getter)]
  pub fn attachment(&self) -> Option<Bytes> {
    self.inner.attachment().cloned().map(Bytes::from_inner)
  }

  /// The source info of this query, or `null` if absent.
  #[napi(getter)]
  pub fn source_info(&self) -> Option<SourceInfo> {
    self
      .inner
      .source_info()
      .cloned()
      .map(SourceInfo::from_inner)
  }

  /// Whether this query accepts replies whose key expression doesn't match it.
  #[napi(getter)]
  pub fn accept_replies(&self) -> ReplyKeyExpr {
    self.inner.accepts_replies().into()
  }

  /// The priority the reply will be sent with (the query's own priority).
  #[napi(getter)]
  pub fn priority(&self) -> Priority {
    self.inner.priority().into()
  }

  /// The congestion control the reply will be routed with.
  #[napi(getter)]
  pub fn congestion_control(&self) -> CongestionControl {
    self.inner.congestion_control().into()
  }

  /// Whether the reply is sent express (not batched).
  #[napi(getter)]
  pub fn express(&self) -> bool {
    self.inner.express()
  }

  /// Replies to this query with a `Put` sample on `keyExpr`.
  ///
  /// By default a query only accepts replies whose key expression intersects
  /// its own (see `acceptReplies`).
  #[napi]
  pub async fn reply(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    payload: Either<String, Uint8Array>,
    options: Option<ReplyOptions>,
  ) -> napi::Result<()> {
    let payload: ZBytes = match payload {
      Either::A(s) => ZBytes::from(s),
      Either::B(bytes) => ZBytes::from(bytes.to_vec()),
    };

    let mut builder = self.inner.reply(key_expr.0, payload);
    if let Some(opts) = options {
      if let Some(encoding) = opts.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(express) = opts.express {
        builder = builder.express(express);
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

  /// Replies to this query with an error payload.
  ///
  /// The error reply is sent with the QoS of the query.
  #[napi]
  pub async fn reply_err(
    &self,
    payload: Either<String, Uint8Array>,
    options: Option<ReplyErrOptions>,
  ) -> napi::Result<()> {
    let payload: ZBytes = match payload {
      Either::A(s) => ZBytes::from(s),
      Either::B(bytes) => ZBytes::from(bytes.to_vec()),
    };

    let mut builder = self.inner.reply_err(payload);
    if let Some(encoding) = options.and_then(|o| o.encoding) {
      builder = builder.encoding(encoding);
    }

    builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Replies to this query with a `Delete` sample on `keyExpr`.
  ///
  /// The reply is sent with the QoS of the query.
  #[napi]
  pub async fn reply_del(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<ReplyDelOptions>,
  ) -> napi::Result<()> {
    let mut builder = self.inner.reply_del(key_expr.0);
    if let Some(opts) = options {
      if let Some(express) = opts.express {
        builder = builder.express(express);
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
}
