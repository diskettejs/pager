use napi_derive::napi;
use zenoh::query::{
  ConsolidationMode as ZConsolidationMode, QueryTarget as ZQueryTarget,
  ReplyKeyExpr as ZReplyKeyExpr,
};

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
