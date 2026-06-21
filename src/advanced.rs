//! Configuration objects for advanced pub/sub.
//!
//! `zenoh-ext`'s advanced pub/sub is integrated directly into [`Publisher`] and
//! [`Subscriber`] rather than exposed as separate `Advanced*` types: every
//! publisher/subscriber is an advanced one. These objects configure the extra
//! capabilities (caching, sample-miss detection, history, recovery, detection)
//! and each maps to the matching `zenoh-ext` builder via `into_zenoh`.
//!
//! [`Publisher`]: crate::publisher::Publisher
//! [`Subscriber`]: crate::subscriber::Subscriber
use std::time::Duration;

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::qos::{CongestionControl, Priority};

/// QoS applied to the samples a publisher's cache sends back when a subscriber
/// queries history or recovers missed samples.
#[napi(object)]
pub struct RepliesConfig {
  /// Priority of reply samples (default: `Data`).
  pub priority: Option<Priority>,
  /// Congestion control for reply samples (default: `Block`).
  pub congestion_control: Option<CongestionControl>,
  /// When `true`, reply samples are sent unbatched.
  pub express: Option<bool>,
}

impl RepliesConfig {
  pub(crate) fn into_zenoh(self) -> zenoh_ext::RepliesConfig {
    let mut config = zenoh_ext::RepliesConfig::default();
    if let Some(priority) = self.priority {
      config = config.priority(priority.into());
    }
    if let Some(congestion_control) = self.congestion_control {
      config = config.congestion_control(congestion_control.into());
    }
    if let Some(express) = self.express {
      config = config.express(express);
    }
    config
  }
}

/// Attaches a cache to a publisher so matching subscribers can recover history
/// and/or missed samples from it.
#[napi(object)]
pub struct CacheConfig {
  /// How many samples to keep per resource (default: 1).
  pub max_samples: Option<u32>,
  /// QoS for replies served from the cache.
  pub replies_config: Option<RepliesConfig>,
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

/// Periodic heartbeat that advertises the last sample's sequence number, letting
/// subscribers detect and recover a lost *last* sample.
#[napi(object)]
pub struct HeartbeatConfig {
  /// Heartbeat period, in milliseconds.
  pub period_ms: u32,
  /// When `true`, the sequence number is advertised only when it changed since
  /// the previous period (`sporadicHeartbeat`); otherwise every period
  /// (`heartbeat`).
  pub sporadic: Option<bool>,
}

/// Enables sample-miss detection on a publisher: each sample is tagged with a
/// per-publisher sequence number, which is what lets subscribers detect misses
/// (`sampleMissListener`) and recover them (`recovery`). The optional
/// `heartbeat` additionally allows the last sample to be recovered.
#[napi(object)]
pub struct MissDetectionConfig {
  /// Periodically advertise the last sample's sequence number.
  pub heartbeat: Option<HeartbeatConfig>,
}

impl MissDetectionConfig {
  pub(crate) fn into_zenoh(self) -> zenoh_ext::MissDetectionConfig {
    let config = zenoh_ext::MissDetectionConfig::default();
    match self.heartbeat {
      Some(heartbeat) => {
        let period = Duration::from_millis(heartbeat.period_ms as u64);
        if heartbeat.sporadic == Some(true) {
          config.sporadic_heartbeat(period)
        } else {
          config.heartbeat(period)
        }
      }
      None => config,
    }
  }
}

/// Enables a subscriber to query for historical samples on startup. History can
/// only be served by publishers that enable `cache`.
#[napi(object)]
pub struct HistoryConfig {
  /// Detect late-joining publishers (via liveliness) and query their history.
  pub detect_late_publishers: Option<bool>,
  /// Query at most this many samples per resource.
  pub max_samples: Option<u32>,
  /// Query only samples no older than this many seconds.
  pub max_age_secs: Option<f64>,
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

/// Configures recovery of detected lost samples. Exactly one mode must be set:
/// `heartbeat` (recover from publisher heartbeats) or `periodicQueriesMs`
/// (recover by polling). They are mutually exclusive — `zenoh-ext` enforces this
/// at the type level, which cannot be expressed in TypeScript, so it is checked
/// at declaration time instead. Recovery can only be achieved by publishers that
/// enable both `cache` and `sampleMissDetection`.
#[napi(object)]
pub struct RecoveryConfig {
  /// Recover by subscribing to publisher heartbeats.
  pub heartbeat: Option<bool>,
  /// Recover by querying for missed samples every this many milliseconds.
  pub periodic_queries_ms: Option<u32>,
}

impl RecoveryConfig {
  pub(crate) fn into_zenoh(self) -> Result<zenoh_ext::RecoveryConfig> {
    let base = zenoh_ext::RecoveryConfig::<false>::default();
    match (self.heartbeat == Some(true), self.periodic_queries_ms) {
      (true, None) => Ok(base.heartbeat()),
      (false, Some(period_ms)) => {
        Ok(base.periodic_queries(Duration::from_millis(period_ms as u64)))
      }
      (true, Some(_)) => Err(Error::from_reason(
        "recovery options `heartbeat` and `periodicQueriesMs` are mutually exclusive",
      )),
      (false, None) => Err(Error::from_reason(
        "recovery requires exactly one of `heartbeat: true` or `periodicQueriesMs`",
      )),
    }
  }
}
