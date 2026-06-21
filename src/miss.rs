use std::future::Future;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::handlers::{DefaultHandler, FifoChannelHandler, RingChannelHandler};

use crate::error::to_napi_err;
use crate::handlers::{self, ChannelHandler, ChannelReceiver, ChannelType};
use crate::session::EntityGlobalId;

/// A report that a subscriber detected missed samples from a source.
///
/// Misses are only detected from publishers that enable `sampleMissDetection`.
#[napi(object)]
pub struct Miss {
  /// The source (publisher entity) the missed samples were from.
  pub source: EntityGlobalId,
  /// How many consecutive samples were missed.
  pub nb: u32,
}

impl Miss {
  fn from_zenoh(miss: zenoh_ext::Miss) -> Self {
    Self {
      source: EntityGlobalId::from_zenoh(miss.source()),
      nb: miss.nb(),
    }
  }
}

type ZenohMiss = zenoh_ext::Miss;
type FifoListener = zenoh_ext::SampleMissListener<Arc<FifoChannelHandler<ZenohMiss>>>;
type RingListener = zenoh_ext::SampleMissListener<Arc<RingChannelHandler<ZenohMiss>>>;

enum SampleMissListenerInner {
  Fifo(FifoListener),
  Ring(RingListener),
}

impl SampleMissListenerInner {
  fn undeclare(self) -> Result<()> {
    use zenoh::Wait;
    match self {
      SampleMissListenerInner::Fifo(listener) => listener.undeclare().wait().map_err(to_napi_err),
      SampleMissListenerInner::Ring(listener) => listener.undeclare().wait().map_err(to_napi_err),
    }
  }
}

/// Notifies of samples a subscriber detected as missed, delivered through a
/// channel. Obtain one from [`Subscriber::sampleMissListener`].
///
/// Consume it with `for await (const miss of listener)`, or pull with
/// `recv()` / `tryRecv()`. The listener is tied to its subscriber: if the
/// subscriber is undeclared or dropped, iteration ends.
#[napi(async_iterator)]
pub struct SampleMissListener {
  inner: Option<SampleMissListenerInner>,
  /// Released together with `inner` on undeclare, so the handler is dropped
  /// exactly as zenoh's own `undeclare` does.
  receiver: Option<ChannelReceiver<ZenohMiss>>,
}

impl SampleMissListener {
  pub(crate) async fn declare(
    builder: zenoh_ext::SampleMissListenerBuilder<'_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let (inner, receiver) = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<ZenohMiss>(capacity);
        let listener = builder.with(handler).await.map_err(to_napi_err)?;
        (SampleMissListenerInner::Fifo(listener), receiver)
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<ZenohMiss>(capacity);
        let listener = builder.with(handler).await.map_err(to_napi_err)?;
        (SampleMissListenerInner::Ring(listener), receiver)
      }
    };
    Ok(Self {
      inner: Some(inner),
      receiver: Some(receiver),
    })
  }
}

#[napi]
impl SampleMissListener {
  /// Wait for the next miss notification, resolving to `null` once the listener
  /// is closed.
  #[napi]
  pub async fn recv(&self) -> Result<Option<Miss>> {
    let receiver = self.receiver.clone();
    match receiver {
      Some(receiver) => Ok(receiver.recv().await.map(Miss::from_zenoh)),
      None => Ok(None),
    }
  }

  /// Return a buffered miss notification if one is immediately available, or
  /// `null` if the channel is currently empty. Throws once the listener has
  /// disconnected, letting a polling loop tell "nothing yet" apart from "closed".
  #[napi]
  pub fn try_recv(&self) -> Result<Option<Miss>> {
    match &self.receiver {
      Some(receiver) => receiver
        .try_recv()
        .map(|miss| miss.map(Miss::from_zenoh))
        .map_err(to_napi_err),
      None => Err(Error::from_reason(
        "sample miss listener has been undeclared",
      )),
    }
  }

  /// Undeclare the listener; any buffered notifications are dropped with the
  /// handler. Resolves synchronously.
  #[napi]
  pub fn undeclare(&mut self) -> Result<()> {
    self.receiver = None;
    match self.inner.take() {
      Some(inner) => inner.undeclare(),
      None => Ok(()),
    }
  }
}

#[napi]
impl AsyncGenerator for SampleMissListener {
  type Yield = Miss;
  type Next = ();
  type Return = ();

  fn next(
    &mut self,
    _value: Option<Self::Next>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let receiver = self.receiver.clone();
    async move {
      match receiver {
        Some(receiver) => Ok(receiver.recv().await.map(Miss::from_zenoh)),
        None => Ok(None),
      }
    }
  }
}
