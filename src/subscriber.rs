use std::future::Future;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::handlers::{DefaultHandler, FifoChannelHandler, RingChannelHandler};
use zenoh::liveliness::LivelinessSubscriberBuilder;
use zenoh_ext::{AdvancedSubscriber, AdvancedSubscriberBuilder};

use crate::advanced::{HistoryConfig, RecoveryConfig};
use crate::error::to_napi_err;
use crate::handlers::{self, ChannelHandler, ChannelReceiver, ChannelType};
use crate::keyexpr::KeyExpr;
use crate::miss::SampleMissListener;
use crate::sample::{Locality, Sample};
use crate::session::EntityGlobalId;

type ZenohSample = zenoh::sample::Sample;
// Regular subscribers are advanced subscribers; liveliness subscribers
// (`Liveliness::declareSubscriber`, `detectPublishers`) are plain — `zenoh-ext`
// has no advanced liveliness subscriber — so both kinds are represented.
type FifoSubscriber = zenoh::pubsub::Subscriber<Arc<FifoChannelHandler<ZenohSample>>>;
type RingSubscriber = zenoh::pubsub::Subscriber<Arc<RingChannelHandler<ZenohSample>>>;
type AdvancedFifoSubscriber = AdvancedSubscriber<Arc<FifoChannelHandler<ZenohSample>>>;
type AdvancedRingSubscriber = AdvancedSubscriber<Arc<RingChannelHandler<ZenohSample>>>;

/// The declared subscriber, kept alive (and undeclarable) regardless of which
/// channel kind backs it and whether it is advanced (regular) or plain
/// (liveliness).
enum SubscriberInner {
  Fifo(FifoSubscriber),
  Ring(RingSubscriber),
  AdvancedFifo(AdvancedFifoSubscriber),
  AdvancedRing(AdvancedRingSubscriber),
}

impl SubscriberInner {
  fn key_expr(&self) -> zenoh::key_expr::KeyExpr<'static> {
    match self {
      SubscriberInner::Fifo(subscriber) => subscriber.key_expr().clone().into_owned(),
      SubscriberInner::Ring(subscriber) => subscriber.key_expr().clone().into_owned(),
      SubscriberInner::AdvancedFifo(subscriber) => subscriber.key_expr().clone().into_owned(),
      SubscriberInner::AdvancedRing(subscriber) => subscriber.key_expr().clone().into_owned(),
    }
  }

  fn id(&self) -> zenoh::session::EntityGlobalId {
    match self {
      SubscriberInner::Fifo(subscriber) => subscriber.id(),
      SubscriberInner::Ring(subscriber) => subscriber.id(),
      SubscriberInner::AdvancedFifo(subscriber) => subscriber.id(),
      SubscriberInner::AdvancedRing(subscriber) => subscriber.id(),
    }
  }

  fn undeclare(self) -> Result<()> {
    use zenoh::Wait;
    match self {
      SubscriberInner::Fifo(subscriber) => subscriber.undeclare().wait().map_err(to_napi_err),
      SubscriberInner::Ring(subscriber) => subscriber.undeclare().wait().map_err(to_napi_err),
      SubscriberInner::AdvancedFifo(subscriber) => {
        subscriber.undeclare().wait().map_err(to_napi_err)
      }
      SubscriberInner::AdvancedRing(subscriber) => {
        subscriber.undeclare().wait().map_err(to_napi_err)
      }
    }
  }
}

/// Options for [`Session::declareSubscriber`].
///
/// Every subscriber is an advanced subscriber: `history`, `recovery`, and
/// `subscriberDetection` configure the advanced capabilities that work with
/// matching advanced publishers.
#[napi(object)]
pub struct SubscriberOptions {
  /// Restrict which publishers' samples are accepted (default: `Any`).
  pub allowed_origin: Option<Locality>,
  /// Channel handler (FIFO or Ring) backing delivery. Defaults to FIFO.
  pub handler: Option<ChannelHandler>,
  /// Query for historical data on startup (served by publishers that `cache`).
  pub history: Option<HistoryConfig>,
  /// Ask for retransmission of detected lost samples (served by publishers that
  /// enable both `cache` and `sampleMissDetection`).
  pub recovery: Option<RecoveryConfig>,
  /// Advertise this subscriber through liveliness so it can be detected.
  pub subscriber_detection: Option<bool>,
  /// Key expression appended to the subscriber-detection liveliness token, used
  /// to convey metadata.
  pub subscriber_detection_metadata: Option<String>,
  /// Timeout for the queries this subscriber issues (history, recovery), in
  /// milliseconds.
  pub query_timeout_ms: Option<u32>,
}

/// A subscriber that delivers [`Sample`]s through a channel.
///
/// Consume it with `for await (const sample of subscriber)`, or pull samples
/// individually with `recv()` / `tryRecv()`. Iteration ends (yields `null`)
/// once the subscriber is undeclared — its buffered samples are dropped with the
/// handler, as in zenoh — or once the session/link closes and any buffered
/// samples have been drained.
#[napi(async_iterator)]
pub struct Subscriber {
  inner: Option<SubscriberInner>,
  /// Released together with `inner` on undeclare, so the handler (and any
  /// samples still buffered in it) is dropped exactly as zenoh's own `undeclare`
  /// does, rather than left draining after the subscriber is gone.
  receiver: Option<ChannelReceiver<ZenohSample>>,
}

impl Subscriber {
  /// Declare an advanced subscriber (the kind every `Session::declareSubscriber`
  /// produces). The builder is already configured with the requested advanced
  /// options; here it is only bound to the chosen channel.
  pub(crate) async fn declare(
    builder: AdvancedSubscriberBuilder<'_, '_, '_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let (inner, receiver) = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<ZenohSample>(capacity);
        let subscriber = builder.with(handler).await.map_err(to_napi_err)?;
        (SubscriberInner::AdvancedFifo(subscriber), receiver)
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<ZenohSample>(capacity);
        let subscriber = builder.with(handler).await.map_err(to_napi_err)?;
        (SubscriberInner::AdvancedRing(subscriber), receiver)
      }
    };
    Ok(Self {
      inner: Some(inner),
      receiver: Some(receiver),
    })
  }

  /// Declare a liveliness subscriber from a [`LivelinessSubscriberBuilder`],
  /// mirroring [`Subscriber::declare`]. A liveliness subscriber delivers the
  /// same [`zenoh::sample::Sample`]s a regular subscriber does (here, the
  /// liveliness changes — `Put` on a token appearing, `Delete` on it vanishing),
  /// so it is backed by exactly the same channel machinery and `Subscriber` type.
  /// It is plain rather than advanced: zenoh-ext has no advanced liveliness
  /// subscriber.
  pub(crate) async fn declare_liveliness(
    builder: LivelinessSubscriberBuilder<'_, '_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let (inner, receiver) = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<ZenohSample>(capacity);
        let subscriber = builder.with(handler).await.map_err(to_napi_err)?;
        (SubscriberInner::Fifo(subscriber), receiver)
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<ZenohSample>(capacity);
        let subscriber = builder.with(handler).await.map_err(to_napi_err)?;
        (SubscriberInner::Ring(subscriber), receiver)
      }
    };
    Ok(Self {
      inner: Some(inner),
      receiver: Some(receiver),
    })
  }

  fn get(&self) -> Result<&SubscriberInner> {
    self
      .inner
      .as_ref()
      .ok_or_else(|| Error::from_reason("subscriber has been undeclared"))
  }
}

#[napi]
impl Subscriber {
  /// Wait for the next sample, resolving to `null` once the subscriber is
  /// undeclared, or once it closes and all buffered samples have been drained.
  #[napi]
  pub async fn recv(&self) -> Result<Option<Sample>> {
    let receiver = self.receiver.clone();
    match receiver {
      Some(receiver) => Ok(receiver.recv().await.map(Sample::new)),
      None => Ok(None),
    }
  }

  /// Return a buffered sample if one is immediately available, or `null` if the
  /// channel is currently empty. Throws once the subscriber has disconnected
  /// (undeclared, or the session closed and all buffered samples drained),
  /// letting a polling loop tell "nothing yet" apart from "closed".
  #[napi]
  pub fn try_recv(&self) -> Result<Option<Sample>> {
    match &self.receiver {
      Some(receiver) => receiver
        .try_recv()
        .map(|sample| sample.map(Sample::new))
        .map_err(to_napi_err),
      None => Err(Error::from_reason("subscriber has been undeclared")),
    }
  }

  /// Undeclare the subscriber. Iteration / `recv` then end and `tryRecv` throws;
  /// any buffered samples are dropped with the handler. Resolves synchronously.
  #[napi]
  pub fn undeclare(&mut self) -> Result<()> {
    // Release the receiver with the declaration: zenoh drops the handler (and
    // anything still buffered in it) as part of undeclaring, so mirror that
    // instead of leaving a FIFO buffer draining after the subscriber is gone.
    self.receiver = None;
    match self.inner.take() {
      Some(inner) => inner.undeclare(),
      None => Ok(()),
    }
  }

  /// Declare a [`SampleMissListener`] for samples this subscriber detected as
  /// missed. The optional channel `handler` (FIFO or Ring) backs the
  /// notifications; defaults to FIFO.
  ///
  /// Misses are only detectable from publishers that enable
  /// `sampleMissDetection`. Not available on liveliness subscribers.
  #[napi]
  pub async fn sample_miss_listener(
    &self,
    handler: Option<ChannelHandler>,
  ) -> Result<SampleMissListener> {
    let builder = match self.get()? {
      SubscriberInner::AdvancedFifo(subscriber) => subscriber.sample_miss_listener(),
      SubscriberInner::AdvancedRing(subscriber) => subscriber.sample_miss_listener(),
      SubscriberInner::Fifo(_) | SubscriberInner::Ring(_) => {
        return Err(Error::from_reason(
          "sample miss detection is not available on liveliness subscribers",
        ));
      }
    };
    SampleMissListener::declare(builder, handler).await
  }

  /// Declare a [`Subscriber`] that detects matching advanced publishers through
  /// liveliness: each sample is a `Put` when a publisher appears and a `Delete`
  /// when it vanishes. The optional channel `handler` defaults to FIFO.
  ///
  /// Only publishers that enable `publisherDetection` can be detected. Not
  /// available on liveliness subscribers.
  #[napi]
  pub async fn detect_publishers(&self, handler: Option<ChannelHandler>) -> Result<Subscriber> {
    let builder = match self.get()? {
      SubscriberInner::AdvancedFifo(subscriber) => subscriber.detect_publishers(),
      SubscriberInner::AdvancedRing(subscriber) => subscriber.detect_publishers(),
      SubscriberInner::Fifo(_) | SubscriberInner::Ring(_) => {
        return Err(Error::from_reason(
          "publisher detection is not available on liveliness subscribers",
        ));
      }
    };
    Subscriber::declare_liveliness(builder, handler).await
  }

  /// The key expression this subscriber is subscribed to.
  #[napi(getter)]
  pub fn key_expr(&self) -> Result<KeyExpr> {
    Ok(KeyExpr::from_zenoh(self.get()?.key_expr()))
  }

  /// This subscriber's globally-unique entity id.
  #[napi(getter)]
  pub fn id(&self) -> Result<EntityGlobalId> {
    Ok(EntityGlobalId::from_zenoh(self.get()?.id()))
  }
}

#[napi]
impl AsyncGenerator for Subscriber {
  type Yield = Sample;
  type Next = ();
  type Return = ();

  fn next(
    &mut self,
    _value: Option<Self::Next>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let receiver = self.receiver.clone();
    async move {
      match receiver {
        Some(receiver) => Ok(receiver.recv().await.map(Sample::new)),
        None => Ok(None),
      }
    }
  }
}
