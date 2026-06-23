use crate::locator::Locator;
use crate::whatami::WhatAmI;
use napi_derive::napi;
use zenoh::scouting::Hello as ZHello;

#[napi]
pub struct Hello {
  pub(crate) inner: ZHello,
}

impl Hello {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZHello) -> Self {
    Hello { inner }
  }
}

#[napi]
impl Hello {
  #[napi]
  pub fn locators(&self) -> Vec<Locator> {
    self
      .inner
      .locators()
      .iter()
      .map(|l| Locator::from_inner(l.clone()))
      .collect()
  }

  #[napi(getter)]
  pub fn whatami(&self) -> WhatAmI {
    self.inner.whatami().into()
  }

  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }
}
