use napi_derive::napi;
use zenoh_ext::Miss as ZMiss;

use crate::entity_global_id::EntityGlobalId;

/// A missed-samples notification, produced by a [`SampleMissListener`]. Sample
/// miss detection requires the matching publisher to enable
/// `sampleMissDetection`.
#[napi]
pub struct Miss {
  inner: ZMiss,
}

impl Miss {
  /// Internal constructor contract: wrap an owned `zenoh-ext` value.
  pub(crate) fn from_inner(inner: ZMiss) -> Self {
    Miss { inner }
  }
}

#[napi]
impl Miss {
  /// The source of the missed samples.
  #[napi(getter)]
  pub fn source(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(self.inner.source())
  }

  /// The number of missed samples.
  #[napi(getter)]
  pub fn nb(&self) -> u32 {
    self.inner.nb()
  }
}
