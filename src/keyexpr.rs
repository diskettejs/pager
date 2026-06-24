use napi::ValueType;
use napi::bindgen_prelude::{FromNapiRef, FromNapiValue, TypeName, sys};
use napi_derive::napi;
use zenoh::key_expr::KeyExpr as ZKeyExpr;

#[napi]
pub struct KeyExpr {
  pub(crate) inner: ZKeyExpr<'static>,
}

impl KeyExpr {
  pub(crate) fn from_inner(inner: ZKeyExpr<'static>) -> Self {
    KeyExpr { inner }
  }
}

#[napi]
impl KeyExpr {
  /// Constructs a key expression, rejecting any string that isn't canon.
  ///
  /// Use `autocanonize` to canonize the input before validating it.
  #[napi(constructor)]
  pub fn new(expr: String) -> napi::Result<Self> {
    let inner: ZKeyExpr<'static> =
      ZKeyExpr::new(expr).map_err(|err| napi::Error::from_reason(err.to_string()))?;
    Ok(Self::from_inner(inner))
  }

  /// Canonizes the passed value before constructing the key expression.
  #[napi(factory)]
  pub fn autocanonize(expr: String) -> napi::Result<Self> {
    let inner: ZKeyExpr<'static> =
      ZKeyExpr::autocanonize(expr).map_err(|err| napi::Error::from_reason(err.to_string()))?;
    Ok(Self::from_inner(inner))
  }

  /// Constructs a key expression from a string. Equivalent to `KeyExpr.new`.
  #[napi(factory)]
  pub fn from_str(expr: String) -> napi::Result<Self> {
    Self::new(expr)
  }

  /// Performs string concatenation and returns the result as a `KeyExpr`.
  ///
  /// You should probably prefer `join` as zenoh may then take
  /// advantage of the hierarchical separation it inserts.
  #[napi]
  pub fn concat(&self, other: String) -> napi::Result<KeyExpr> {
    let inner = self
      .inner
      .concat(&other)
      .map_err(|err| napi::Error::from_reason(err.to_string()))?;
    Ok(Self::from_inner(inner))
  }

  /// Joins both sides, inserting a `/` in between them.
  ///
  /// This should be your preferred method when concatenating path segments.
  #[napi]
  pub fn join(&self, other: String) -> napi::Result<KeyExpr> {
    let inner = self
      .inner
      .join(&other)
      .map_err(|err| napi::Error::from_reason(err.to_string()))?;
    Ok(Self::from_inner(inner))
  }

  /// The canonical string form of this key expression.
  #[napi(getter)]
  pub fn as_str(&self) -> String {
    self.inner.as_str().to_string()
  }

  /// Returns `true` if the key expressions intersect, i.e. there exists at
  /// least one key contained in both of the sets defined by `self` and `other`.
  #[napi]
  pub fn intersects(&self, #[napi(ts_arg_type = "string | KeyExpr")] other: KeyExprArg) -> bool {
    self.inner.as_keyexpr().intersects(other.0.as_keyexpr())
  }

  /// Returns `true` if `self` includes `other`, i.e. the set defined by `self`
  /// contains every key belonging to the set defined by `other`.
  #[napi]
  pub fn includes(&self, #[napi(ts_arg_type = "string | KeyExpr")] other: KeyExprArg) -> bool {
    self.inner.as_keyexpr().includes(other.0.as_keyexpr())
  }

  /// Returns `true` if `self` contains any wildcard character (`**` or `$*`).
  #[napi(getter)]
  pub fn is_wild(&self) -> bool {
    self.inner.as_keyexpr().is_wild()
  }
}

/// Owned input form of [`KeyExpr`] for use at call sites.
pub struct KeyExprArg(pub(crate) ZKeyExpr<'static>);

impl FromNapiValue for KeyExprArg {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
    // Distinguish a JS string from a `KeyExpr` class instance (an object) by
    // inspecting the value's runtime type.
    let value_type = napi::type_of!(env, napi_val)?;
    match value_type {
      ValueType::String => {
        let expr = unsafe { String::from_napi_value(env, napi_val)? };
        let inner: ZKeyExpr<'static> =
          ZKeyExpr::new(expr).map_err(|err| napi::Error::from_reason(err.to_string()))?;
        Ok(Self(inner))
      }
      _ => {
        let instance = unsafe { <KeyExpr as FromNapiRef>::from_napi_ref(env, napi_val)? };
        Ok(Self(instance.inner.clone().into_owned()))
      }
    }
  }
}

impl TypeName for KeyExprArg {
  fn type_name() -> &'static str {
    "KeyExpr"
  }

  fn value_type() -> ValueType {
    ValueType::Unknown
  }
}
