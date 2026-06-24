use std::time::{Duration, SystemTime};

use napi::ValueType;
use napi::bindgen_prelude::{BigInt, FromNapiRef, FromNapiValue, TypeName, sys};
use napi_derive::napi;
use zenoh::query::{TimeBound as ZTimeBound, TimeRange as ZTimeRange};
use zenoh::time::Timestamp as ZTimestamp;

#[napi]
pub struct Timestamp {
  pub(crate) inner: ZTimestamp,
}

impl Timestamp {
  pub(crate) fn from_inner(inner: ZTimestamp) -> Self {
    Timestamp { inner }
  }
}

// NOTE: `Timestamp::new(time: NTP64, id: ID)` is intentionally not exposed.
// Both arguments are uhlc-internal types with no natural JS representation,
// making the raw constructor awkward to call from JS. Use `parseRfc3339`
// (or a `Timestamp` produced by zenoh, e.g. a sample's timestamp) instead.

#[napi]
impl Timestamp {
  /// Parse an RFC3339 time representation (`<rfc3339>/<hlc_id_hex>`) into a
  /// `Timestamp`.
  #[napi(factory)]
  pub fn parse_rfc3339(s: String) -> napi::Result<Self> {
    ZTimestamp::parse_rfc3339(&s)
      .map(Self::from_inner)
      .map_err(|e| napi::Error::from_reason(e.cause))
  }

  /// Convert to an RFC3339 time representation with nanoseconds precision.
  /// e.g.: `"2024-07-01T13:51:12.129693000Z/33"`.
  #[napi]
  pub fn to_string_rfc3339_lossy(&self) -> String {
    self.inner.to_string_rfc3339_lossy()
  }

  /// Returns the NTP64 time as its raw `u64` representation.
  #[napi]
  pub fn get_time(&self) -> BigInt {
    BigInt::from(self.inner.get_time().as_u64())
  }

  /// Returns the HLC's unique `id` as a hexadecimal string.
  #[napi]
  pub fn get_id(&self) -> String {
    self.inner.get_id().to_string()
  }

  /// Returns the time difference from `other` in milliseconds.
  #[napi]
  pub fn get_diff_duration(&self, other: &Timestamp) -> f64 {
    self.inner.get_diff_duration(&other.inner).as_secs_f64() * 1000.0
  }
}

/// Owned input form of [`Timestamp`] for use as an options field.
pub struct TimestampArg(pub(crate) ZTimestamp);

impl FromNapiValue for TimestampArg {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
    let instance = unsafe { <Timestamp as FromNapiRef>::from_napi_ref(env, napi_val)? };
    Ok(Self(instance.inner))
  }
}

impl TypeName for TimestampArg {
  fn type_name() -> &'static str {
    "Timestamp"
  }

  fn value_type() -> ValueType {
    ValueType::Object
  }
}

#[napi]
pub struct TimeRange {
  pub(crate) inner: ZTimeRange,
}

/// Best-effort string form of a `TimeBound<TimeExpr>`. `Unbounded` maps to
/// `None` (JS `null`); inclusive/exclusive bounds render their `TimeExpr`.
fn bound_to_string(bound: &ZTimeBound<zenoh::query::TimeExpr>) -> Option<String> {
  match bound {
    ZTimeBound::Inclusive(t) | ZTimeBound::Exclusive(t) => Some(t.to_string()),
    ZTimeBound::Unbounded => None,
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
