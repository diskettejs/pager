use napi::bindgen_prelude::*;

/// Map a Zenoh error into a NAPI `Error` that rejects the JS Promise, tagging it
/// with the operation that failed.
pub(crate) fn zerr(context: &str, e: impl std::fmt::Display) -> Error {
  Error::new(Status::GenericFailure, format!("{context}: {e}"))
}
