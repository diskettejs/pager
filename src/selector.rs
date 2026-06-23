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
