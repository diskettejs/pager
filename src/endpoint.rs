use std::str::FromStr;

use napi_derive::napi;
use zenoh::config::EndPoint as ZEndPoint;

use crate::locator::Locator;
use crate::metadata::Metadata;

#[napi]
pub struct EndPoint {
  pub(crate) inner: ZEndPoint,
}

impl EndPoint {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZEndPoint) -> Self {
    EndPoint { inner }
  }
}

/// The four string components of an [`EndPoint`], as returned by
/// [`EndPoint::split`].
#[napi(object)]
pub struct EndPointParts {
  pub protocol: String,
  pub address: String,
  pub metadata: String,
  pub config: String,
}

#[napi]
impl EndPoint {
  /// Parses an endpoint from its canonical string form
  /// `<protocol>/<address>[?<metadata>][#<config>]`.
  #[napi(constructor)]
  pub fn new(s: String) -> napi::Result<Self> {
    let inner = ZEndPoint::from_str(&s).map_err(|err| napi::Error::from_reason(err.to_string()))?;
    Ok(Self::from_inner(inner))
  }

  /// The protocol of this endpoint.
  #[napi(getter)]
  pub fn protocol(&self) -> String {
    self.inner.protocol().as_str().to_string()
  }

  /// The address of this endpoint.
  #[napi(getter)]
  pub fn address(&self) -> String {
    self.inner.address().as_str().to_string()
  }

  /// The canonical string form of this endpoint.
  #[napi(getter)]
  pub fn as_str(&self) -> String {
    self.inner.as_str().to_string()
  }

  /// The metadata view of this endpoint.
  #[napi]
  pub fn metadata(&self) -> Metadata {
    Metadata::from_inner(self.inner.clone())
  }

  /// The config substring of this endpoint in its canonical string form.
  #[napi]
  pub fn config(&self) -> String {
    self.inner.config().as_str().to_string()
  }

  /// Splits this endpoint into its `protocol`, `address`, `metadata` and
  /// `config` string components.
  #[napi]
  pub fn split(&self) -> EndPointParts {
    let (protocol, address, metadata, config) = self.inner.split();
    EndPointParts {
      protocol: protocol.as_str().to_string(),
      address: address.as_str().to_string(),
      metadata: metadata.as_str().to_string(),
      config: config.as_str().to_string(),
    }
  }

  /// Demotes this endpoint to a [`Locator`], dropping any config component.
  #[napi]
  pub fn to_locator(&self) -> Locator {
    Locator::from_inner(self.inner.to_locator())
  }
}
