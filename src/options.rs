use std::time::Duration;

use napi::bindgen_prelude::Uint8Array;
use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::cancellation::CancellationTokenArg;
use crate::handlers::ChannelConfig;
use crate::keyexpr::KeyExprArg;
use crate::parameters::ParametersArg;
use crate::qos::{CongestionControl, Locality, Priority, Reliability};
use crate::query::{ConsolidationMode, QueryTarget, ReplyKeyExpr};
use crate::reply::RepliesConfig;
use crate::source_info::SourceInfoArg;
use crate::time::TimestampArg;

#[napi(object, object_to_js = false)]
pub struct HistoryConfig {
  pub detect_late_publishers: Option<bool>,
  pub max_samples: Option<u32>,
  pub max_age_secs: Option<f64>,
}

// Recovery is modeled as a discriminated union, mirroring zenoh's `RecoveryConfig`
// type-state where periodic-queries and heartbeat are mutually exclusive and the
// query period is intrinsic to the periodic-queries mode. Each arm carries its own
// single-variant `mode` tag so napi emits an exact string literal (`'PeriodicQueries'`
// / `'Heartbeat'`), which both discriminates the `Either` at runtime and yields a
// precise TS union. There is therefore no representable "both", "neither", or
// "periodic-queries without a period" state — no resolution logic is needed.

#[napi(string_enum)]
pub enum PeriodicQueriesMode {
  PeriodicQueries,
}

#[napi(string_enum)]
pub enum HeartbeatMode {
  Heartbeat,
}

/// Recover missed samples by periodically querying for them.
#[napi(object, object_to_js = false)]
pub struct PeriodicQueriesRecovery {
  pub mode: PeriodicQueriesMode,
  /// Query period in milliseconds.
  pub period_ms: u32,
}

/// Recover the last missed sample by subscribing to publisher heartbeats.
#[napi(object, object_to_js = false)]
pub struct HeartbeatRecovery {
  pub mode: HeartbeatMode,
}

pub(crate) fn recovery_into_zenoh(
  recovery: Either<PeriodicQueriesRecovery, HeartbeatRecovery>,
) -> zenoh_ext::RecoveryConfig {
  let config = zenoh_ext::RecoveryConfig::<false>::default();
  match recovery {
    Either::A(periodic) => {
      config.periodic_queries(Duration::from_millis(periodic.period_ms as u64))
    }
    Either::B(_heartbeat) => config.heartbeat(),
  }
}

#[napi(object, object_to_js = false)]
pub struct CacheConfig {
  pub max_samples: Option<u32>,
  pub replies_config: Option<RepliesConfig>,
}

#[napi(object, object_to_js = false)]
pub struct HeartbeatConfig {
  pub period_ms: u32,
  pub sporadic: Option<bool>,
}

#[napi(object, object_to_js = false)]
pub struct MissDetectionConfig {
  pub heartbeat: Option<HeartbeatConfig>,
}

impl HistoryConfig {
  pub(crate) fn into_zenoh(self) -> zenoh_ext::HistoryConfig {
    let mut config = zenoh_ext::HistoryConfig::default();
    if self.detect_late_publishers == Some(true) {
      config = config.detect_late_publishers();
    }
    if let Some(max_samples) = self.max_samples {
      config = config.max_samples(max_samples as usize);
    }
    if let Some(max_age_secs) = self.max_age_secs {
      config = config.max_age(max_age_secs);
    }
    config
  }
}

impl CacheConfig {
  pub(crate) fn into_zenoh(self) -> zenoh_ext::CacheConfig {
    let mut config = zenoh_ext::CacheConfig::default();
    if let Some(max_samples) = self.max_samples {
      config = config.max_samples(max_samples as usize);
    }
    if let Some(replies_config) = self.replies_config {
      config = config.replies_config(replies_config.into_zenoh());
    }
    config
  }
}

impl MissDetectionConfig {
  pub(crate) fn into_zenoh(self) -> zenoh_ext::MissDetectionConfig {
    let mut config = zenoh_ext::MissDetectionConfig::default();
    if let Some(heartbeat) = self.heartbeat {
      let period = Duration::from_millis(heartbeat.period_ms as u64);
      // `heartbeat` and `sporadic_heartbeat` are mutually exclusive in zenoh.
      config = if heartbeat.sporadic == Some(true) {
        config.sporadic_heartbeat(period)
      } else {
        config.heartbeat(period)
      };
    }
    config
  }
}

/// Options for `Session.put` — mirrors `SessionPutBuilder`.
#[napi(object, object_to_js = false)]
pub struct PutOptions {
  pub encoding: Option<String>,
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub reliability: Option<Reliability>,
  pub allowed_destination: Option<Locality>,
  #[napi(ts_type = "Timestamp")]
  pub timestamp: Option<TimestampArg>,
  pub attachment: Option<Uint8Array>,
  #[napi(ts_type = "SourceInfo")]
  pub source_info: Option<SourceInfoArg>,
}

/// Options for `Session.delete` — mirrors `SessionDeleteBuilder`.
#[napi(object, object_to_js = false)]
pub struct DeleteOptions {
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub reliability: Option<Reliability>,
  pub allowed_destination: Option<Locality>,
  #[napi(ts_type = "Timestamp")]
  pub timestamp: Option<TimestampArg>,
  pub attachment: Option<Uint8Array>,
  #[napi(ts_type = "SourceInfo")]
  pub source_info: Option<SourceInfoArg>,
}

/// Options for `Session.get` — mirrors `SessionGetBuilder`.
#[napi(object, object_to_js = false)]
pub struct GetOptions {
  pub target: Option<QueryTarget>,
  pub consolidation: Option<ConsolidationMode>,
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub allowed_destination: Option<Locality>,
  /// Timeout in milliseconds.
  pub timeout: Option<f64>,
  pub payload: Option<Uint8Array>,
  pub encoding: Option<String>,
  pub attachment: Option<Uint8Array>,
  #[napi(ts_type = "SourceInfo")]
  pub source_info: Option<SourceInfoArg>,
  #[napi(ts_type = "CancellationToken")]
  pub cancellation_token: Option<CancellationTokenArg>,
}

/// Options for `Session.declarePublisher` — mirrors `PublisherBuilder`.
#[napi(object, object_to_js = false)]
pub struct PublisherOptions {
  pub encoding: Option<String>,
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub reliability: Option<Reliability>,
  pub allowed_destination: Option<Locality>,
  pub cache: Option<CacheConfig>,
  pub sample_miss_detection: Option<MissDetectionConfig>,
  pub publisher_detection: Option<bool>,
  // TODO: this should also accept KeyExpr
  pub publisher_detection_metadata: Option<String>,
}

/// Options for `Publisher.put` — mirrors `AdvancedPublisherPutBuilder`.
///
/// QoS is fixed by the publisher; only per-publication fields appear here. The
/// advanced builder manages `source_info` itself (for sample-miss sequencing)
/// and exposes no setter for it, so it is intentionally absent.
#[napi(object, object_to_js = false)]
pub struct PublisherPutOptions {
  pub encoding: Option<String>,
  #[napi(ts_type = "Timestamp")]
  pub timestamp: Option<TimestampArg>,
  pub attachment: Option<Uint8Array>,
}

/// Options for `Publisher.delete` — mirrors `AdvancedPublisherDeleteBuilder`.
///
/// As with `PublisherPutOptions`, `source_info` is managed by the advanced
/// builder and has no setter.
#[napi(object, object_to_js = false)]
pub struct PublisherDeleteOptions {
  #[napi(ts_type = "Timestamp")]
  pub timestamp: Option<TimestampArg>,
  pub attachment: Option<Uint8Array>,
}

/// Options for `Session.declareSubscriber` — mirrors `SubscriberBuilder`.
#[napi(object, object_to_js = false)]
pub struct SubscriberOptions {
  pub allowed_origin: Option<Locality>,
  /// Channel selection for the subscription's handler (default: FIFO).
  pub handler: Option<ChannelConfig>,
  pub history: Option<HistoryConfig>,
  #[napi(ts_type = "PeriodicQueriesRecovery | HeartbeatRecovery")]
  pub recovery: Option<Either<PeriodicQueriesRecovery, HeartbeatRecovery>>,
  pub subscriber_detection: Option<bool>,
  // TODO: this should also accept KeyExpr
  pub subscriber_detection_metadata: Option<String>,
  pub query_timeout_ms: Option<f64>,
}

/// Options for `Publisher.matchingListener` — selects the notification channel.
#[napi(object, object_to_js = false)]
pub struct MatchingListenerOptions {
  /// Channel selection for the listener's handler (default: FIFO).
  pub handler: Option<ChannelConfig>,
}

/// Options for `Subscriber.sampleMissListener` — selects the notification channel.
#[napi(object, object_to_js = false)]
pub struct SampleMissListenerOptions {
  /// Channel selection for the listener's handler (default: FIFO).
  pub handler: Option<ChannelConfig>,
}

/// Options for `Session.declareQueryable` — mirrors `QueryableBuilder`.
#[napi(object, object_to_js = false)]
pub struct QueryableOptions {
  pub complete: Option<bool>,
  pub allowed_origin: Option<Locality>,
}

/// Options for `Session.declareQuerier` — mirrors `QuerierBuilder`.
#[napi(object, object_to_js = false)]
pub struct QuerierOptions {
  pub target: Option<QueryTarget>,
  pub consolidation: Option<ConsolidationMode>,
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub allowed_destination: Option<Locality>,
  /// Timeout in milliseconds.
  pub timeout: Option<f64>,
  pub accept_replies: Option<ReplyKeyExpr>,
}

/// Options for `Querier.get` — mirrors `QuerierGetBuilder`.
#[napi(object, object_to_js = false)]
pub struct QuerierGetOptions {
  #[napi(ts_type = "string | Parameters")]
  pub parameters: Option<ParametersArg>,
  pub payload: Option<Uint8Array>,
  pub encoding: Option<String>,
  pub attachment: Option<Uint8Array>,
  #[napi(ts_type = "SourceInfo")]
  pub source_info: Option<SourceInfoArg>,
  #[napi(ts_type = "CancellationToken")]
  pub cancellation_token: Option<CancellationTokenArg>,
  /// Channel selection for the reply handler (default: FIFO).
  pub handler: Option<ChannelConfig>,
}

/// Options for `Query.reply` — mirrors `ReplyBuilder`.
#[napi(object, object_to_js = false)]
pub struct ReplyOptions {
  pub encoding: Option<String>,
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  #[napi(ts_type = "Timestamp")]
  pub timestamp: Option<TimestampArg>,
  pub attachment: Option<Uint8Array>,
  #[napi(ts_type = "SourceInfo")]
  pub source_info: Option<SourceInfoArg>,
}

/// Options for `Liveliness.declareSubscriber` — mirrors `LivelinessSubscriberBuilder`.
#[napi(object, object_to_js = false)]
pub struct LivelinessSubscriberOptions {
  pub history: Option<bool>,
  /// Channel selection for the subscriber's handler (default: FIFO).
  pub handler: Option<ChannelConfig>,
}

/// Options for `Liveliness.get` — mirrors `LivelinessGetBuilder`.
#[napi(object, object_to_js = false)]
pub struct LivelinessGetOptions {
  /// Timeout in milliseconds.
  pub timeout: Option<f64>,
  #[napi(ts_type = "CancellationToken")]
  pub cancellation_token: Option<CancellationTokenArg>,
}
