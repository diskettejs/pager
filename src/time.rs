use napi::ValueType;
use napi::bindgen_prelude::{BigInt, FromNapiRef, FromNapiValue, TypeName, sys};
use napi_derive::napi;
use zenoh::time::Timestamp as ZTimestamp;

#[napi]
pub struct Timestamp {
  pub(crate) inner: ZTimestamp,
}

impl Timestamp {
  /// Internal constructor contract: wrap an owned `zenoh` value.
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
  /// [`Timestamp`].
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
///
/// Copies the inner `zenoh` value out at unwrap time, so it can be carried
/// across an `.await` without borrowing the JS class instance.
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
