use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::keyexpr::KeyExpr;
use crate::qos::{CongestionControl, Priority, Reliability};
use crate::session::EntityGlobalId;
use crate::time::Timestamp;

/// Restricts which entities a publication reaches (or a subscription accepts),
/// by whether they live in the same session or elsewhere.
#[napi(string_enum)]
pub enum Locality {
  /// Only entities in the same session.
  SessionLocal,
  /// Only remote entities (not in the same session).
  Remote,
  /// Both local and remote entities (the default).
  Any,
}

impl From<Locality> for zenoh::sample::Locality {
  fn from(value: Locality) -> Self {
    match value {
      Locality::SessionLocal => zenoh::sample::Locality::SessionLocal,
      Locality::Remote => zenoh::sample::Locality::Remote,
      Locality::Any => zenoh::sample::Locality::Any,
    }
  }
}

/// Whether a sample was produced by a `put` or a `delete`.
#[napi(string_enum)]
pub enum SampleKind {
  /// Issued by a `put`.
  Put,
  /// Issued by a `delete`.
  Delete,
}

impl From<zenoh::sample::SampleKind> for SampleKind {
  fn from(value: zenoh::sample::SampleKind) -> Self {
    match value {
      zenoh::sample::SampleKind::Put => SampleKind::Put,
      zenoh::sample::SampleKind::Delete => SampleKind::Delete,
    }
  }
}

/// Source metadata for a publication: which entity produced the sample and the
/// source's own sequence number for it. Used by advanced pub/sub (e.g. for
/// missing-sample detection); the base primitives just transmit it.
#[napi(object)]
pub struct SourceInfo {
  /// Id of the entity that produced the sample.
  pub source_id: EntityGlobalId,
  /// The source's sequence number for this sample.
  pub source_sn: u32,
}

impl SourceInfo {
  pub(crate) fn from_zenoh(info: &zenoh::sample::SourceInfo) -> Self {
    Self {
      source_id: EntityGlobalId::from_zenoh(*info.source_id()),
      source_sn: info.source_sn(),
    }
  }

  pub(crate) fn to_zenoh(&self) -> Result<zenoh::sample::SourceInfo> {
    Ok(zenoh::sample::SourceInfo::new(
      self.source_id.to_zenoh()?,
      self.source_sn,
    ))
  }
}

/// A data sample received by a subscriber (or a query reply): the payload plus
/// all of its metadata.
///
/// Fields are exposed as lazy getters; the payload is only copied into a
/// `Buffer` when accessed.
#[napi]
pub struct Sample {
  inner: zenoh::sample::Sample,
}

impl Sample {
  pub(crate) fn new(inner: zenoh::sample::Sample) -> Self {
    Self { inner }
  }
}

#[napi]
impl Sample {
  /// The key expression this sample was published on.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_zenoh(self.inner.key_expr().clone().into_owned())
  }

  /// The payload bytes.
  #[napi(getter)]
  pub fn payload(&self) -> Buffer {
    Buffer::from(self.inner.payload().to_bytes().to_vec())
  }

  /// Whether this sample is a `Put` or a `Delete`.
  #[napi(getter)]
  pub fn kind(&self) -> SampleKind {
    self.inner.kind().into()
  }

  /// The payload encoding.
  #[napi(getter)]
  pub fn encoding(&self) -> String {
    self.inner.encoding().to_string()
  }

  /// The timestamp attached to the sample, if any.
  #[napi(getter)]
  pub fn timestamp(&self) -> Option<Timestamp> {
    self.inner.timestamp().map(Timestamp::from_zenoh)
  }

  /// The congestion control strategy the sample was sent with.
  #[napi(getter)]
  pub fn congestion_control(&self) -> CongestionControl {
    self.inner.congestion_control().into()
  }

  /// The priority the sample was sent with.
  #[napi(getter)]
  pub fn priority(&self) -> Priority {
    self.inner.priority().into()
  }

  /// Whether the sample was sent express (unbatched).
  #[napi(getter)]
  pub fn express(&self) -> bool {
    self.inner.express()
  }

  /// The delivery reliability the sample was sent with.
  #[napi(getter)]
  pub fn reliability(&self) -> Reliability {
    self.inner.reliability().into()
  }

  /// The attachment bytes, if any.
  #[napi(getter)]
  pub fn attachment(&self) -> Option<Buffer> {
    self
      .inner
      .attachment()
      .map(|attachment| Buffer::from(attachment.to_bytes().to_vec()))
  }

  /// The source metadata, if any.
  #[napi(getter)]
  pub fn source_info(&self) -> Option<SourceInfo> {
    self.inner.source_info().map(SourceInfo::from_zenoh)
  }
}
