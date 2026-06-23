use napi::ValueType;
use napi::bindgen_prelude::{FromNapiRef, FromNapiValue, TypeName, ValidateNapiValue, sys};
use napi_derive::napi;
use zenoh::query::Selector as ZSelector;

use crate::keyexpr::{KeyExpr, KeyExprArg};
use crate::parameters::Parameters;

#[napi]
pub struct Selector {
  pub(crate) inner: ZSelector<'static>,
}

impl Selector {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZSelector<'static>) -> Self {
    Selector { inner }
  }
}

/// The deconstructed string parts of a [`Selector`].
#[napi(object)]
pub struct SelectorParts {
  pub key_expr: String,
  pub parameters: String,
}

#[napi]
impl Selector {
  /// Builds a selector from a key expression and optional parameters.
  ///
  /// Both inputs are owned, yielding a `Selector<'static>`.
  #[napi(constructor)]
  pub fn new(
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    parameters: Option<String>,
  ) -> Self {
    let inner = ZSelector::owned(key_expr.0, parameters.unwrap_or_default());
    Self::from_inner(inner)
  }

  /// The key expression part of this selector.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.inner.key_expr().clone().into_owned())
  }

  /// The parameters part of this selector.
  #[napi(getter)]
  pub fn parameters(&self) -> Parameters {
    Parameters::from_inner(self.inner.parameters().clone().into_owned())
  }

  /// Deconstructs the selector into its key expression and parameters strings.
  #[napi]
  pub fn split(&self) -> SelectorParts {
    SelectorParts {
      key_expr: self.inner.key_expr().as_str().to_string(),
      parameters: self.inner.parameters().as_str().to_string(),
    }
  }
}

/// Owned input form of [`Selector`] for use at call sites (e.g. `Session.get`).
///
/// Accepts a plain JS string (parsed via zenoh's `key/expr?params` split), a
/// [`KeyExpr`] class instance (an empty-parameter selector), or a [`Selector`]
/// class instance (cloned). It always owns a `Selector<'static>`, so it never
/// borrows the JS class instance and can be carried across an `.await`.
pub struct SelectorArg(pub(crate) ZSelector<'static>);

impl FromNapiValue for SelectorArg {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
    let value_type = napi::type_of!(env, napi_val)?;
    match value_type {
      ValueType::String => {
        let s = unsafe { String::from_napi_value(env, napi_val)? };
        let inner = ZSelector::try_from(s).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Self(inner))
      }
      // Both `Selector` and `KeyExpr` instances are objects; an `instanceof`
      // check on the registered constructor tells them apart. A `KeyExpr`
      // becomes a selector with empty parameters.
      _ if unsafe { <&Selector as ValidateNapiValue>::validate(env, napi_val) }.is_ok() => {
        let instance = unsafe { <Selector as FromNapiRef>::from_napi_ref(env, napi_val)? };
        Ok(Self(instance.inner.clone()))
      }
      _ => {
        let instance = unsafe { <KeyExpr as FromNapiRef>::from_napi_ref(env, napi_val)? };
        Ok(Self(ZSelector::from(instance.inner.clone().into_owned())))
      }
    }
  }
}

impl TypeName for SelectorArg {
  fn type_name() -> &'static str {
    "Selector"
  }

  fn value_type() -> ValueType {
    ValueType::Unknown
  }
}
