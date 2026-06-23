use napi_derive::napi;
use zenoh::config::Locator as ZLocator;

use crate::endpoint::EndPoint;
use crate::metadata::Metadata;

#[napi]
pub struct Locator {
  pub(crate) inner: ZLocator,
}

impl Locator {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZLocator) -> Self {
    Locator { inner }
  }
}

#[napi]
impl Locator {
  /// Constructs a locator from its `<protocol>`, `<address>` and `<metadata>`
  /// parts.
  #[napi(constructor)]
  pub fn new(protocol: String, address: String, metadata: String) -> napi::Result<Self> {
    let inner = ZLocator::new(protocol, address, metadata)
      .map_err(|err| napi::Error::from_reason(err.to_string()))?;
    Ok(Self::from_inner(inner))
  }

  /// The protocol of this locator.
  #[napi(getter)]
  pub fn protocol(&self) -> String {
    self.inner.protocol().as_str().to_string()
  }

  /// The address of this locator.
  #[napi(getter)]
  pub fn address(&self) -> String {
    self.inner.address().as_str().to_string()
  }

  /// The canonical string form of this locator.
  #[napi(getter)]
  pub fn as_str(&self) -> String {
    self.inner.as_str().to_string()
  }

  /// The metadata view of this locator.
  #[napi]
  pub fn metadata(&self) -> Metadata {
    Metadata::from_inner(self.inner.to_endpoint())
  }

  /// Promotes this locator to an [`EndPoint`].
  #[napi]
  pub fn to_endpoint(&self) -> EndPoint {
    EndPoint::from_inner(self.inner.to_endpoint())
  }
}
