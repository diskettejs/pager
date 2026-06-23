use napi_derive::napi;
use zenoh::query::ReplyError as ZReplyError;

use crate::bytes::Bytes;
use crate::encoding::Encoding;

#[napi]
pub struct ReplyError {
  pub(crate) inner: ZReplyError,
}

impl ReplyError {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZReplyError) -> Self {
    ReplyError { inner }
  }
}

#[napi]
impl ReplyError {
  #[napi(getter)]
  pub fn encoding(&self) -> Encoding {
    Encoding::from_inner(self.inner.encoding().clone())
  }

  #[napi(getter)]
  pub fn payload(&self) -> Bytes {
    Bytes::from_inner(self.inner.payload().clone())
  }
}
