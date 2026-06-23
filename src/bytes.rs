use napi::bindgen_prelude::Uint8Array;
use napi_derive::napi;
use zenoh::bytes::ZBytes;

#[napi]
pub struct Bytes {
  inner: ZBytes,
}

impl Bytes {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZBytes) -> Self {
    Bytes { inner }
  }

  /// Internal: clone the underlying `ZBytes`. Cheap — `ZBytes` is backed by
  /// reference-counted buffers.
  pub(crate) fn clone_inner(&self) -> ZBytes {
    self.inner.clone()
  }
}

#[napi]
impl Bytes {
  #[napi]
  pub fn new() -> Self {
    Bytes {
      inner: ZBytes::new(),
    }
  }

  #[napi(factory)]
  pub fn from_bytes(data: Uint8Array) -> Self {
    Self::from_inner(ZBytes::from(data.to_vec()))
  }

  #[napi(factory)]
  pub fn from_string(value: String) -> Self {
    Self::from_inner(ZBytes::from(value))
  }

  #[napi(getter)]
  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  #[napi(getter)]
  pub fn len(&self) -> u32 {
    self.inner.len() as u32
  }

  #[napi]
  pub fn to_bytes(&self) -> Uint8Array {
    Uint8Array::from(self.inner.to_bytes().into_owned())
  }

  #[napi]
  pub fn to_string(&self) -> napi::Result<String> {
    self
      .inner
      .try_to_string()
      .map(|s| s.into_owned())
      .map_err(|err| napi::Error::from_reason(err.to_string()))
  }
}
