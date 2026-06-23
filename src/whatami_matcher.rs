use crate::whatami::WhatAmI;
use napi_derive::napi;
use zenoh::config::WhatAmIMatcher as ZWhatAmIMatcher;

#[napi]
pub struct WhatAmIMatcher {
  pub(crate) inner: ZWhatAmIMatcher,
}

impl WhatAmIMatcher {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZWhatAmIMatcher) -> Self {
    WhatAmIMatcher { inner }
  }
}

#[napi]
impl WhatAmIMatcher {
  #[napi(factory)]
  pub fn empty() -> Self {
    Self::from_inner(ZWhatAmIMatcher::empty())
  }

  #[napi]
  pub fn router(&self) -> Self {
    Self::from_inner(self.inner.router())
  }

  #[napi]
  pub fn peer(&self) -> Self {
    Self::from_inner(self.inner.peer())
  }

  #[napi]
  pub fn client(&self) -> Self {
    Self::from_inner(self.inner.client())
  }

  #[napi(getter)]
  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  #[napi]
  pub fn matches(&self, w: WhatAmI) -> bool {
    self.inner.matches(w.into())
  }

  #[napi]
  pub fn to_str(&self) -> String {
    self.inner.to_str().to_string()
  }
}
