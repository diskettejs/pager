use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::to_napi_err;

/// A Zenoh key expression: a `/`-separated expression that addresses a set of
/// keys (mirrors `zenoh::key_expr::KeyExpr`).
///
/// To be valid a key expression must be *canon*. Build one from a string with
/// the constructor — which rejects non-canon input — or with
/// [`KeyExpr::autocanonize`], which canonizes the input first.
///
/// Anywhere this library accepts a key expression you may pass either a
/// `string` or a `KeyExpr`; the getters that expose one hand back a `KeyExpr`.
#[napi]
#[derive(Clone)]
pub struct KeyExpr {
  pub(crate) inner: zenoh::key_expr::KeyExpr<'static>,
}

impl KeyExpr {
  pub(crate) fn from_zenoh(inner: zenoh::key_expr::KeyExpr<'static>) -> Self {
    Self { inner }
  }
}

#[napi]
impl KeyExpr {
  /// Build a key expression from a string, failing if it is not a valid, canon
  /// key expression. Use [`KeyExpr::autocanonize`] to canonize automatically.
  #[napi(constructor)]
  pub fn new(key_expr: String) -> Result<Self> {
    let inner = zenoh::key_expr::KeyExpr::try_from(key_expr).map_err(to_napi_err)?;
    Ok(Self { inner })
  }

  /// Build a key expression from a string, canonizing it first. Fails only if
  /// the value is not a valid key expression even after canonization.
  #[napi(factory)]
  pub fn autocanonize(key_expr: String) -> Result<Self> {
    let inner = zenoh::key_expr::KeyExpr::autocanonize(key_expr).map_err(to_napi_err)?;
    Ok(Self { inner })
  }

  /// Whether this key expression and `other` share at least one matching key.
  #[napi]
  pub fn intersects(&self, #[napi(ts_arg_type = "string | KeyExpr")] other: KeyExprArg) -> bool {
    self.inner.intersects(&other.0)
  }

  /// Whether every key matched by `other` is also matched by this expression.
  #[napi]
  pub fn includes(&self, #[napi(ts_arg_type = "string | KeyExpr")] other: KeyExprArg) -> bool {
    self.inner.includes(&other.0)
  }

  /// Whether this key expression is equal to `other`.
  #[napi]
  pub fn equals(&self, #[napi(ts_arg_type = "string | KeyExpr")] other: KeyExprArg) -> bool {
    self.inner == other.0
  }

  /// Join this key expression with `other`, inserting a `/` between them.
  /// Prefer this over [`KeyExpr::concat`] so Zenoh can exploit the separation.
  #[napi]
  pub fn join(&self, other: String) -> Result<KeyExpr> {
    let inner = self.inner.join(&other).map_err(to_napi_err)?;
    Ok(Self { inner })
  }

  /// Concatenate `other` onto this key expression with no separator inserted.
  #[napi]
  pub fn concat(&self, other: String) -> Result<KeyExpr> {
    let inner = self.inner.concat(&other).map_err(to_napi_err)?;
    Ok(Self { inner })
  }

  /// The canon string form of this key expression.
  // An inherent `to_string` is what surfaces as JS `toString()`; the
  // `Display`-based alternative clippy suggests is not callable from napi.
  #[napi]
  #[allow(clippy::inherent_to_string)]
  pub fn to_string(&self) -> String {
    self.inner.as_str().to_string()
  }
}

/// A key expression argument accepted from JS as either a `string` or a
/// [`KeyExpr`] instance, eagerly converted into an owned `zenoh` key expression
/// at the FFI boundary.
///
/// Converting up front (rather than holding a borrowed `&KeyExpr`) is what makes
/// this safe to use in the `async` session methods: napi-rs only keeps a class
/// reference alive across an `await` when it is a top-level argument, not when
/// it is nested inside a wrapper like this one. A borrowed `&KeyExpr` could
/// therefore dangle if the JS object were garbage-collected mid-await; owning
/// the value here sidesteps that entirely.
pub struct KeyExprArg(pub(crate) zenoh::key_expr::KeyExpr<'static>);

impl FromNapiValue for KeyExprArg {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> Result<Self> {
    let mut value_type: sys::napi_valuetype = 0;
    check_status!(
      unsafe { sys::napi_typeof(env, napi_val, &mut value_type) },
      "Failed to read the type of a key expression argument",
    )?;
    let inner = match ValueType::from(value_type) {
      ValueType::String => {
        let key_expr = unsafe { String::from_napi_value(env, napi_val)? };
        zenoh::key_expr::KeyExpr::try_from(key_expr).map_err(to_napi_err)?
      }
      // Reject anything that is not a `KeyExpr` instance before unwrapping:
      // `from_napi_value` for `&KeyExpr` is a raw unwrap that would otherwise
      // reinterpret some other wrapped class's pointer as a `KeyExpr`.
      _ => {
        unsafe { <&KeyExpr as ValidateNapiValue>::validate(env, napi_val)? };
        let key_expr = unsafe { <&KeyExpr as FromNapiValue>::from_napi_value(env, napi_val)? };
        key_expr.inner.clone()
      }
    };
    Ok(Self(inner))
  }
}
