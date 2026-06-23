use napi::ValueType;
use napi::bindgen_prelude::{FromNapiRef, FromNapiValue, TypeName, sys};
use napi_derive::napi;
use zenoh::query::Parameters as ZParameters;

#[napi]
pub struct Parameters {
  pub(crate) inner: ZParameters<'static>,
}

impl Parameters {
  /// Internal constructor contract: wrap an owned `zenoh` value.
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
  ///
  /// The owned `String` yields a `Parameters<'static>`.
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
///
/// Accepts either a plain JS string (in the `a=b;c=d|e;f=g` format) or a
/// [`Parameters`] class instance (whose inner value is cloned). It always owns
/// a `Parameters<'static>`, so it never borrows the JS class instance and can
/// be carried across an `.await`.
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
