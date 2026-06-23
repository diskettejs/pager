use napi_derive::napi;
use zenoh::sample::{Sample as ZSample, SampleFields as ZSampleFields, SampleKind as ZSampleKind};

use crate::bytes::Bytes;
use crate::encoding::Encoding;
use crate::keyexpr::KeyExpr;
use crate::qos::{CongestionControl, Priority, Reliability};
use crate::source_info::SourceInfo;
use crate::time::Timestamp;

#[napi(string_enum)]
pub enum SampleKind {
  Put,
  Delete,
}

impl From<ZSampleKind> for SampleKind {
  fn from(kind: ZSampleKind) -> Self {
    match kind {
      ZSampleKind::Put => SampleKind::Put,
      ZSampleKind::Delete => SampleKind::Delete,
    }
  }
}

impl From<SampleKind> for ZSampleKind {
  fn from(kind: SampleKind) -> Self {
    match kind {
      SampleKind::Put => ZSampleKind::Put,
      SampleKind::Delete => ZSampleKind::Delete,
    }
  }
}

#[napi]
pub struct Sample {
  inner: ZSampleFields,
}

impl Sample {
  pub(crate) fn new(zsample: ZSample) -> Self {
    let fields: ZSampleFields = zsample.into();
    Sample { inner: fields }
  }
}

#[napi]
impl Sample {
  #[napi(getter)]
  pub fn payload(&self) -> Bytes {
    Bytes::from_inner(self.inner.payload.clone())
  }

  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.inner.key_expr.clone())
  }

  #[napi(getter)]
  pub fn kind(&self) -> SampleKind {
    self.inner.kind.into()
  }

  #[napi(getter)]
  pub fn encoding(&self) -> Encoding {
    Encoding::from_inner(self.inner.encoding.clone())
  }

  #[napi(getter)]
  pub fn timestamp(&self) -> Option<Timestamp> {
    self.inner.timestamp.map(Timestamp::from_inner)
  }

  #[napi(getter)]
  pub fn express(&self) -> bool {
    self.inner.express
  }

  #[napi(getter)]
  pub fn priority(&self) -> Priority {
    self.inner.priority.into()
  }

  #[napi(getter)]
  pub fn congestion_control(&self) -> CongestionControl {
    self.inner.congestion_control.into()
  }

  #[napi(getter)]
  pub fn reliability(&self) -> Reliability {
    self.inner.reliability.into()
  }

  #[napi(getter)]
  pub fn attachment(&self) -> Option<Bytes> {
    self.inner.attachment.clone().map(Bytes::from_inner)
  }

  #[napi(getter)]
  pub fn source_info(&self) -> Option<SourceInfo> {
    self.inner.source_info.clone().map(SourceInfo::from_inner)
  }
}
