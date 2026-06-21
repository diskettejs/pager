use std::str::FromStr;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::session::ZenohId;

use crate::error::to_napi_err;

/// A Zenoh timestamp: a hybrid-logical-clock time plus the id of the clock that
/// produced it. Obtain one from [`Session::newTimestamp`]; it can then be
/// passed back via the `timestamp` publication option.
#[napi(object)]
pub struct Timestamp {
  /// The NTP64-encoded time component (a 64-bit value).
  pub time: BigInt,
  /// The id of the source clock, as a hex string (a Zenoh ID).
  pub id: String,
}

impl Timestamp {
  /// Snapshot a `zenoh::time::Timestamp` into the JS-facing representation.
  pub(crate) fn from_zenoh(timestamp: &zenoh::time::Timestamp) -> Self {
    Self {
      time: BigInt::from(timestamp.get_time().as_u64()),
      id: timestamp.get_id().to_string(),
    }
  }

  /// Rebuild a `zenoh::time::Timestamp`. Fails if `id` is not a valid Zenoh ID.
  pub(crate) fn to_zenoh(&self) -> Result<zenoh::time::Timestamp> {
    let id: zenoh::time::TimestampId = ZenohId::from_str(&self.id).map_err(to_napi_err)?.into();
    Ok(zenoh::time::Timestamp::new(
      zenoh::time::NTP64(self.time.get_u64().1),
      id,
    ))
  }
}
