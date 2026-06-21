use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh_ext::AdvancedPublisher;

use crate::advanced::{CacheConfig, MissDetectionConfig};
use crate::bytes::to_zbytes;
use crate::error::to_napi_err;
use crate::handlers::ChannelHandler;
use crate::keyexpr::KeyExpr;
use crate::macros::apply_options;
use crate::matching::{MatchingListener, MatchingStatus};
use crate::qos::{CongestionControl, Priority, Reliability};
use crate::sample::Locality;
use crate::session::EntityGlobalId;
use crate::time::Timestamp;

/// Options for [`Session::declarePublisher`]. These settings are fixed for the
/// publisher's lifetime; per-publication `put`/`delete` may only override
/// payload-level fields (encoding, attachment, …), not QoS.
///
/// Every publisher is an advanced publisher: `cache`, `sampleMissDetection`, and
/// `publisherDetection` configure the advanced capabilities that matching
/// subscribers rely on for history, recovery, and detection.
#[napi(object)]
pub struct PublisherOptions {
  /// Default encoding for publications.
  pub encoding: Option<String>,
  /// Congestion control strategy (default: `Drop`).
  pub congestion_control: Option<CongestionControl>,
  /// Priority of publications (default: `Data`).
  pub priority: Option<Priority>,
  /// When `true`, messages are sent unbatched, trading throughput for latency.
  pub express: Option<bool>,
  /// Delivery reliability (default: `Reliable`).
  pub reliability: Option<Reliability>,
  /// Restrict which matching subscribers receive the data (default: `Any`).
  pub allowed_destination: Option<Locality>,
  /// Cache recent samples so matching subscribers can recover history and/or
  /// missed samples from this publisher.
  pub cache: Option<CacheConfig>,
  /// Tag samples with sequence numbers so matching subscribers can detect (and,
  /// with `cache`, recover) lost samples.
  pub sample_miss_detection: Option<MissDetectionConfig>,
  /// Advertise this publisher (via liveliness) so subscribers can detect it and
  /// request its history.
  pub publisher_detection: Option<bool>,
  /// Key expression appended to the publisher-detection liveliness token and the
  /// cache queryable, used to convey metadata.
  pub publisher_detection_metadata: Option<String>,
}

/// Options for [`Publisher::put`].
#[napi(object)]
pub struct PublisherPutOptions {
  /// Encoding of this payload, overriding the publisher's default.
  pub encoding: Option<String>,
  /// Optional attachment carried alongside the payload.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// Timestamp to attach; obtain one from [`Session::newTimestamp`]. Overrides
  /// the timestamp the publisher attaches automatically.
  pub timestamp: Option<Timestamp>,
}

/// Options for [`Publisher::delete`].
#[napi(object)]
pub struct PublisherDeleteOptions {
  /// Optional attachment carried alongside the deletion.
  pub attachment: Option<Either<String, Uint8Array>>,
  /// Timestamp to attach; obtain one from [`Session::newTimestamp`]. Overrides
  /// the timestamp the publisher attaches automatically.
  pub timestamp: Option<Timestamp>,
}

/// A publisher bound to a key expression, with QoS fixed at declaration time.
/// Create one with [`Session::declarePublisher`].
///
/// Every publisher is an advanced publisher (see [`PublisherOptions`]); the
/// publisher owns sequencing, so `put`/`delete` no longer take a `sourceInfo`.
#[napi]
pub struct Publisher {
  inner: Option<AdvancedPublisher<'static>>,
  // `AdvancedPublisher` exposes no `reliability()` getter, so the value chosen at
  // declaration is stashed here to keep the getter. Stored as the zenoh `Copy`
  // type so the getter can hand back a fresh value.
  reliability: zenoh::qos::Reliability,
}

impl Publisher {
  pub(crate) fn new(
    inner: AdvancedPublisher<'static>,
    reliability: zenoh::qos::Reliability,
  ) -> Self {
    Self {
      inner: Some(inner),
      reliability,
    }
  }

  fn get(&self) -> Result<&AdvancedPublisher<'static>> {
    self
      .inner
      .as_ref()
      .ok_or_else(|| Error::from_reason("publisher has been undeclared"))
  }
}

#[napi]
impl Publisher {
  /// Publish a `Put` sample to this publisher's key expression.
  #[napi]
  pub async fn put(
    &self,
    payload: Either<String, Uint8Array>,
    options: Option<PublisherPutOptions>,
  ) -> Result<()> {
    let publisher = self.get()?;
    let mut builder = publisher.put(to_zbytes(payload));
    if let Some(options) = options {
      apply_options!(builder, options, {
        encoding,
        attachment => zbytes,
        timestamp => try_zenoh,
      });
    }
    builder.await.map_err(to_napi_err)
  }

  /// Publish a `Delete` sample to this publisher's key expression.
  #[napi]
  pub async fn delete(&self, options: Option<PublisherDeleteOptions>) -> Result<()> {
    let publisher = self.get()?;
    let mut builder = publisher.delete();
    if let Some(options) = options {
      apply_options!(builder, options, {
        attachment => zbytes,
        timestamp => try_zenoh,
      });
    }
    builder.await.map_err(to_napi_err)
  }

  /// Whether any subscribers currently match this publisher's key expression.
  #[napi]
  pub async fn matching_status(&self) -> Result<MatchingStatus> {
    let publisher = self.get()?;
    let status = publisher.matching_status().await.map_err(to_napi_err)?;
    Ok(MatchingStatus {
      matching: status.matching(),
    })
  }

  /// Declare a [`MatchingListener`] that notifies when this publisher's set of
  /// matching subscribers changes. The optional channel `handler` (FIFO or
  /// Ring) backs the notifications; defaults to FIFO.
  #[napi]
  pub async fn matching_listener(
    &self,
    handler: Option<ChannelHandler>,
  ) -> Result<MatchingListener> {
    let builder = self.get()?.matching_listener();
    MatchingListener::declare(builder, handler).await
  }

  /// Undeclare this publisher. Subsequent operations on it will error.
  ///
  /// Resolves synchronously, so awaiting the returned value is optional.
  #[napi]
  pub fn undeclare(&mut self) -> Result<()> {
    use zenoh::Wait;
    match self.inner.take() {
      Some(publisher) => publisher.undeclare().wait().map_err(to_napi_err),
      None => Ok(()),
    }
  }

  /// The key expression this publisher publishes to.
  #[napi(getter)]
  pub fn key_expr(&self) -> Result<KeyExpr> {
    Ok(KeyExpr::from_zenoh(
      self.get()?.key_expr().clone().into_owned(),
    ))
  }

  /// The default encoding applied to publications.
  #[napi(getter)]
  pub fn encoding(&self) -> Result<String> {
    Ok(self.get()?.encoding().to_string())
  }

  /// The congestion control strategy.
  #[napi(getter)]
  pub fn congestion_control(&self) -> Result<CongestionControl> {
    Ok(self.get()?.congestion_control().into())
  }

  /// The publication priority.
  #[napi(getter)]
  pub fn priority(&self) -> Result<Priority> {
    Ok(self.get()?.priority().into())
  }

  /// The delivery reliability.
  #[napi(getter)]
  pub fn reliability(&self) -> Result<Reliability> {
    // Touch `inner` so an undeclared publisher errors here too, matching the
    // other getters.
    self.get()?;
    Ok(self.reliability.into())
  }

  /// This publisher's globally-unique entity id.
  #[napi(getter)]
  pub fn id(&self) -> Result<EntityGlobalId> {
    Ok(EntityGlobalId::from_zenoh(self.get()?.id()))
  }
}
