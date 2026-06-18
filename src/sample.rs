use crate::enums::{CongestionControl, Priority, SampleKind};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Zenoh `Timestamp` as plain data. Convert to a JS `Date` with the
/// `ntp64ToDate(ntp64)` helper (lives in the JS layer) to keep `Sample` a POJO.
#[napi(object)]
pub struct Timestamp {
  /// zid of the timestamp source.
  pub id: String,
  /// Raw NTP64.
  pub ntp64: BigInt,
}

/// A sample delivered to JS. `#[napi(object)]` makes it a plain JS object, mapped
/// field by field from Zenoh's getters. Only `unstable`-gated fields
/// (`reliability`, `sourceInfo`) are omitted.
#[napi(object)]
pub struct Sample {
  pub key_expr: String,
  pub payload: Uint8Array,
  pub kind: SampleKind,
  pub encoding: String,
  pub timestamp: Option<Timestamp>,
  pub congestion_control: CongestionControl,
  pub priority: Priority,
  pub express: bool,
  pub attachment: Option<Uint8Array>,
}

pub(crate) fn to_js_sample(s: zenoh::sample::Sample) -> Sample {
  let timestamp = s.timestamp().map(|ts| Timestamp {
    id: ts.get_id().to_string(),
    ntp64: BigInt::from(ts.get_time().as_u64()),
  });
  let attachment = s
    .attachment()
    .map(|a| Uint8Array::from(a.to_bytes().into_owned()));

  Sample {
    key_expr: s.key_expr().to_string(),
    payload: Uint8Array::from(s.payload().to_bytes().into_owned()),
    kind: s.kind().into(),
    encoding: s.encoding().to_string(),
    timestamp,
    congestion_control: s.congestion_control().into(),
    priority: s.priority().into(),
    express: s.express(),
    attachment,
  }
}
