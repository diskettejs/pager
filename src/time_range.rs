use std::time::{Duration, SystemTime};

use napi_derive::napi;
use zenoh::query::{TimeBound, TimeRange as ZTimeRange};

#[napi]
pub struct TimeRange {
  pub(crate) inner: ZTimeRange,
}

impl TimeRange {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZTimeRange) -> Self {
    TimeRange { inner }
  }
}

/// Best-effort string form of a `TimeBound<TimeExpr>`. `Unbounded` maps to
/// `None` (JS `null`); inclusive/exclusive bounds render their `TimeExpr`.
fn bound_to_string(bound: &TimeBound<zenoh::query::TimeExpr>) -> Option<String> {
  match bound {
    TimeBound::Inclusive(t) | TimeBound::Exclusive(t) => Some(t.to_string()),
    TimeBound::Unbounded => None,
  }
}

#[napi]
impl TimeRange {
  /// The start bound as a string (the time expression), or `null` if the range
  /// is unbounded at the start.
  #[napi(getter)]
  pub fn start(&self) -> Option<String> {
    bound_to_string(&self.inner.start)
  }

  /// The end bound as a string (the time expression), or `null` if the range is
  /// unbounded at the end.
  #[napi(getter)]
  pub fn end(&self) -> Option<String> {
    bound_to_string(&self.inner.end)
  }

  /// Returns `true` if the given instant (UNIX epoch milliseconds, e.g. from
  /// `Date.now()`) belongs to this range.
  ///
  /// If the bounds contain an "offset" time expression (`now(...)`), this
  /// resolves them against the current system time on each call.
  #[napi]
  pub fn contains(&self, epoch_millis: f64) -> bool {
    let instant = SystemTime::UNIX_EPOCH + Duration::from_secs_f64(epoch_millis / 1000.0);
    self.inner.contains(instant)
  }

  // DEFERRED: `resolve(self) -> TimeRange<SystemTime>` and
  // `resolve_at(self, now: SystemTime) -> TimeRange<SystemTime>`. Both consume
  // the range and change the generic to `TimeRange<SystemTime>`, which this
  // napi class (wrapping `TimeRange<TimeExpr>`) cannot represent. `contains`
  // already performs the equivalent resolution internally, covering the common
  // use case.
}
