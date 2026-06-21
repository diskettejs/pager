use napi::bindgen_prelude::*;

/// Convert any `Display` error into a `napi::Error` so it surfaces as a thrown
/// JS `Error`.
///
/// `zenoh::Error` is a boxed `dyn std::error::Error`, which is `Display`, so
/// `result.map_err(to_napi_err)` is the standard way to bridge a `ZResult`.
pub(crate) fn to_napi_err<E: std::fmt::Display>(err: E) -> Error {
  Error::from_reason(err.to_string())
}
