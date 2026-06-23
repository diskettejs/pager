use napi_derive::napi;
use zenoh::session::EntityGlobalId as ZEntityGlobalId;

#[napi]
pub struct EntityGlobalId {
  pub(crate) inner: ZEntityGlobalId,
}

impl EntityGlobalId {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZEntityGlobalId) -> Self {
    EntityGlobalId { inner }
  }
}

#[napi]
impl EntityGlobalId {
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  #[napi(getter)]
  pub fn eid(&self) -> u32 {
    self.inner.eid()
  }
}
