use napi_derive::napi;
use zenoh::matching::MatchingStatus as ZMatchingStatus;

#[napi]
pub struct MatchingStatus {
  pub(crate) inner: ZMatchingStatus,
}

impl MatchingStatus {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZMatchingStatus) -> Self {
    MatchingStatus { inner }
  }
}

#[napi]
impl MatchingStatus {
  #[napi(getter)]
  pub fn matching(&self) -> bool {
    self.inner.matching()
  }
}
