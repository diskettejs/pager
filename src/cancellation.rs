use napi::ValueType;
use napi::bindgen_prelude::{FromNapiRef, FromNapiValue, TypeName, sys};
use napi_derive::napi;
use zenoh::cancellation::CancellationToken as ZCancellationToken;

#[napi]
pub struct CancellationToken {
  pub(crate) inner: ZCancellationToken,
}

#[napi]
impl CancellationToken {
  #[napi(constructor)]
  pub fn new() -> Self {
    CancellationToken {
      inner: ZCancellationToken::default(),
    }
  }

  /// Interrupt all associated get queries.
  ///
  /// Resolves once cancellation completes. On failure, some operations might
  /// not be cancelled.
  #[napi]
  pub async fn cancel(&self) -> napi::Result<()> {
    // Clone the inner token so we don't borrow `&self` across the await.
    let token = self.inner.clone();
    token
      .cancel()
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Returns true if the token was cancelled (i.e. `cancel` was called).
  #[napi(getter)]
  pub fn is_cancelled(&self) -> bool {
    self.inner.is_cancelled()
  }
}

/// Owned input form of [`CancellationToken`] for use as an options field.
///
/// `CancellationToken` is cheaply clonable (it shares the underlying state), so
/// the arg holds an owned clone that can cross an `.await`.
pub struct CancellationTokenArg(pub(crate) ZCancellationToken);

impl FromNapiValue for CancellationTokenArg {
  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
    let instance = unsafe { <CancellationToken as FromNapiRef>::from_napi_ref(env, napi_val)? };
    Ok(Self(instance.inner.clone()))
  }
}

impl TypeName for CancellationTokenArg {
  fn type_name() -> &'static str {
    "CancellationToken"
  }

  fn value_type() -> ValueType {
    ValueType::Object
  }
}
