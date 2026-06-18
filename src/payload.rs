use napi::bindgen_prelude::*;

/// Native payload boundary: `string | Uint8Array`. Strings are UTF-8 encoded;
/// typed arrays / buffers are taken as raw bytes (a Node `Buffer` is a
/// `Uint8Array`, so it is accepted too). The spec's `ArrayBuffer` arm is widened
/// in the JS wrapper layer (`new Uint8Array(ab)`) — it borrows the V8 scope and so
/// can't cross NAPI's async boundary directly.
pub(crate) fn to_zbytes(input: Either<String, Uint8Array>) -> zenoh::bytes::ZBytes {
  match input {
    Either::A(s) => zenoh::bytes::ZBytes::from(s),
    Either::B(bytes) => zenoh::bytes::ZBytes::from(bytes.to_vec()),
  }
}
