use napi_derive::napi;
use zenoh::liveliness::LivelinessToken as ZLivelinessToken;

#[napi]
pub struct LivelinessToken {
  pub(crate) inner: Option<ZLivelinessToken>,
}

impl LivelinessToken {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZLivelinessToken) -> Self {
    LivelinessToken { inner: Some(inner) }
  }
}

#[napi]
impl LivelinessToken {
  /// Undeclare this liveliness token. `undeclare(self)` consumes the token, so
  /// the owned value is `.take()`n out of the `Option` before awaiting. If the
  /// token was already undeclared (or dropped), this is a no-op.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(token) => token
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      None => Ok(()),
    }
  }
}
