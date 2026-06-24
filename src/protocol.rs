use std::str::FromStr;

use napi::ValueType;
use napi::bindgen_prelude::{FromNapiRef, FromNapiValue, TypeName, sys};
use napi_derive::napi;
use zenoh::config::{
  EndPoint as ZEndPoint, Locator as ZLocator, WhatAmI as ZWhatAmI,
  WhatAmIMatcher as ZWhatAmIMatcher,
};
use zenoh::query::Parameters as ZParameters;

#[napi]
pub struct EndPoint {
  pub(crate) inner: ZEndPoint,
}

impl EndPoint {
  pub(crate) fn from_inner(inner: ZEndPoint) -> Self {
    EndPoint { inner }
  }
}

/// The four string components of an `EndPoint`, as returned by
/// `split`.
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

  /// Demotes this endpoint to a `Locator`, dropping any config component.
  #[napi]
  pub fn to_locator(&self) -> Locator {
    Locator::from_inner(self.inner.to_locator())
  }
}

#[napi]
pub struct Parameters {
  pub(crate) inner: ZParameters<'static>,
}

impl Parameters {
  pub(crate) fn from_inner(inner: ZParameters<'static>) -> Self {
    Parameters { inner }
  }
}

#[napi]
impl Parameters {
  /// Creates empty parameters.
  #[napi(factory)]
  pub fn empty() -> Self {
    Self::from_inner(ZParameters::empty())
  }

  /// Parses parameters from a string in the `a=b;c=d|e;f=g` format.
  #[napi(constructor)]
  pub fn new(params: String) -> Self {
    Self::from_inner(ZParameters::from(params))
  }

  /// Returns the parameters as their canonical string form.
  #[napi(getter)]
  pub fn as_str(&self) -> String {
    self.inner.as_str().to_string()
  }

  /// Returns `true` if the parameters do not contain anything.
  #[napi(getter)]
  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  /// Returns `true` if all keys are sorted in alphabetical order.
  #[napi(getter)]
  pub fn is_ordered(&self) -> bool {
    self.inner.is_ordered()
  }

  /// Returns `true` if the parameters contain the specified key.
  #[napi]
  pub fn contains_key(&self, key: String) -> bool {
    self.inner.contains_key(key)
  }

  /// Returns the value corresponding to the key, if present.
  #[napi]
  pub fn get(&self, key: String) -> Option<String> {
    self.inner.get(key).map(|value| value.to_string())
  }

  /// Returns the values corresponding to the key.
  #[napi]
  pub fn values(&self, key: String) -> Vec<String> {
    self
      .inner
      .values(key)
      .map(|value| value.to_string())
      .collect()
  }

  /// Inserts a key-value pair, returning the previous value if the key was
  /// already present.
  #[napi]
  pub fn insert(&mut self, key: String, value: String) -> Option<String> {
    self.inner.insert(key, value)
  }

  /// Removes a key, returning its value if the key was present.
  #[napi]
  pub fn remove(&mut self, key: String) -> Option<String> {
    self.inner.remove(key)
  }

  /// Extends these parameters with the entries of `other`.
  #[napi]
  pub fn extend(&mut self, other: &Parameters) {
    self.inner.extend(&other.inner);
  }
}

/// Owned input form of [`Parameters`] for use as an options field.
pub struct ParametersArg(pub(crate) ZParameters<'static>);

impl FromNapiValue for ParametersArg {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
    // Distinguish a JS string from a `Parameters` class instance (an object) by
    // inspecting the value's runtime type.
    let value_type = napi::type_of!(env, napi_val)?;
    match value_type {
      ValueType::String => {
        let params = unsafe { String::from_napi_value(env, napi_val)? };
        Ok(Self(ZParameters::from(params)))
      }
      _ => {
        let instance = unsafe { <Parameters as FromNapiRef>::from_napi_ref(env, napi_val)? };
        Ok(Self(instance.inner.clone()))
      }
    }
  }
}

impl TypeName for ParametersArg {
  fn type_name() -> &'static str {
    "Parameters"
  }

  fn value_type() -> ValueType {
    ValueType::Unknown
  }
}

#[napi]
pub struct Locator {
  pub(crate) inner: ZLocator,
}

impl Locator {
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

  /// Promotes this locator to an `EndPoint`.
  #[napi]
  pub fn to_endpoint(&self) -> EndPoint {
    EndPoint::from_inner(self.inner.to_endpoint())
  }
}

#[napi]
pub struct Metadata {
  pub(crate) inner: ZEndPoint,
}

impl Metadata {
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

#[napi(string_enum)]
pub enum WhatAmI {
  Router,
  Peer,
  Client,
}

impl From<WhatAmI> for ZWhatAmI {
  fn from(value: WhatAmI) -> Self {
    match value {
      WhatAmI::Router => Self::Router,
      WhatAmI::Peer => Self::Peer,
      WhatAmI::Client => Self::Client,
    }
  }
}

impl From<ZWhatAmI> for WhatAmI {
  fn from(value: ZWhatAmI) -> Self {
    match value {
      ZWhatAmI::Router => Self::Router,
      ZWhatAmI::Peer => Self::Peer,
      ZWhatAmI::Client => Self::Client,
    }
  }
}

#[napi]
pub struct WhatAmIMatcher {
  pub(crate) inner: ZWhatAmIMatcher,
}

impl WhatAmIMatcher {
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
