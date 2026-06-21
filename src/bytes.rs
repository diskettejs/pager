use napi::bindgen_prelude::*;
use zenoh::bytes::ZBytes;

/// Convert a JS payload into Zenoh's `ZBytes`.
///
/// Accepts a `string` (encoded as UTF-8) or a `Uint8Array`/`Buffer` (copied
/// into an owned buffer). Zero-copy input is a later refinement.
pub(crate) fn to_zbytes(value: Either<String, Uint8Array>) -> ZBytes {
  match value {
    Either::A(string) => ZBytes::from(string),
    Either::B(bytes) => ZBytes::from(bytes.to_vec()),
  }
}
