use napi_derive::napi;
use zenoh::config::EndPoint as ZEndPoint;

#[napi]
pub struct Metadata {
  pub(crate) inner: ZEndPoint,
}

impl Metadata {
  /// Internal constructor contract: own the endpoint backing the metadata view.
  pub(crate) fn from_inner(inner: ZEndPoint) -> Self {
    Metadata { inner }
  }
}

#[napi]
impl Metadata {
  // zenoh's `Metadata` type (which carries these as associated consts) is not
  // re-exported through any path reachable from the `zenoh` crate, so the canon
  // key strings are mirrored here verbatim from
  // `zenoh_protocol::core::endpoint::Metadata`.

  /// The metadata key for reliability (`"rel"`).
  #[napi]
  pub fn reliability_key() -> String {
    "rel".to_string()
  }

  /// The metadata key for priorities (`"prio"`).
  #[napi]
  pub fn priorities_key() -> String {
    "prio".to_string()
  }

  /// The metadata key for multistream (`"multistream"`).
  #[napi]
  pub fn multistream_key() -> String {
    "multistream".to_string()
  }

  /// The metadata key for mixed reliability (`"mixed_rel"`).
  #[napi]
  pub fn mixed_reliability_key() -> String {
    "mixed_rel".to_string()
  }

  /// The metadata substring in its canonical string form.
  #[napi]
  pub fn as_str(&self) -> String {
    self.inner.metadata().as_str().to_string()
  }

  /// Returns `true` if there is no metadata.
  #[napi]
  pub fn is_empty(&self) -> bool {
    self.inner.metadata().is_empty()
  }

  /// Returns the first value associated with `key`, if any.
  #[napi]
  pub fn get(&self, key: String) -> Option<String> {
    self
      .inner
      .metadata()
      .get(&key)
      .map(|value| value.to_string())
  }

  /// Returns every value associated with `key`.
  #[napi]
  pub fn values(&self, key: String) -> Vec<String> {
    self
      .inner
      .metadata()
      .values(&key)
      .map(|value| value.to_string())
      .collect()
  }
}
