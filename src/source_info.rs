use napi::ValueType;
use napi::bindgen_prelude::{FromNapiRef, FromNapiValue, TypeName, sys};
use napi_derive::napi;
use zenoh::sample::SourceInfo as ZSourceInfo;

use crate::session::EntityGlobalId;

#[napi]
pub struct SourceInfo {
  pub(crate) inner: ZSourceInfo,
}

impl SourceInfo {
  pub(crate) fn from_inner(inner: ZSourceInfo) -> Self {
    SourceInfo { inner }
  }
}

#[napi]
impl SourceInfo {
  #[napi(constructor)]
  pub fn new(source_id: &EntityGlobalId, source_sn: u32) -> Self {
    SourceInfo {
      inner: ZSourceInfo::new(source_id.inner, source_sn),
    }
  }

  #[napi(getter)]
  pub fn source_id(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(*self.inner.source_id())
  }

  #[napi(getter)]
  pub fn source_sn(&self) -> u32 {
    self.inner.source_sn()
  }
}

/// Owned input form of [`SourceInfo`] for use as an options field.
pub struct SourceInfoArg(pub(crate) ZSourceInfo);

impl FromNapiValue for SourceInfoArg {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
    let instance = unsafe { <SourceInfo as FromNapiRef>::from_napi_ref(env, napi_val)? };
    Ok(Self(instance.inner.clone()))
  }
}

impl TypeName for SourceInfoArg {
  fn type_name() -> &'static str {
    "SourceInfo"
  }

  fn value_type() -> ValueType {
    ValueType::Object
  }
}
