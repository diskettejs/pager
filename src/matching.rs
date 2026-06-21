use std::future::Future;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::handlers::{DefaultHandler, FifoChannelHandler, RingChannelHandler};
use zenoh::matching::MatchingListenerBuilder;

use crate::error::to_napi_err;
use crate::handlers::{self, ChannelHandler, ChannelReceiver, ChannelType};

/// Whether a publisher currently has matching subscribers (or, later, a querier
/// has matching queryables).
#[napi(object)]
pub struct MatchingStatus {
  /// `true` if at least one matching entity currently exists.
  pub matching: bool,
}

impl MatchingStatus {
  fn from_zenoh(status: zenoh::matching::MatchingStatus) -> Self {
    Self {
      matching: status.matching(),
    }
  }
}

type ZenohStatus = zenoh::matching::MatchingStatus;
type FifoListener = zenoh::matching::MatchingListener<Arc<FifoChannelHandler<ZenohStatus>>>;
type RingListener = zenoh::matching::MatchingListener<Arc<RingChannelHandler<ZenohStatus>>>;

enum MatchingListenerInner {
  Fifo(FifoListener),
  Ring(RingListener),
}

impl MatchingListenerInner {
  fn undeclare(self) -> Result<()> {
    use zenoh::Wait;
    match self {
      MatchingListenerInner::Fifo(listener) => listener.undeclare().wait().map_err(to_napi_err),
      MatchingListenerInner::Ring(listener) => listener.undeclare().wait().map_err(to_napi_err),
    }
  }
}

/// Notifies of changes to a publisher's [`MatchingStatus`], delivered through a
/// channel. Obtain one from [`Publisher::matchingListener`].
///
/// Consume it with `for await (const status of listener)`, or pull with
/// `recv()` / `tryRecv()`. The listener is tied to its publisher: if the
/// publisher is undeclared or dropped, iteration ends.
#[napi(async_iterator)]
pub struct MatchingListener {
  inner: Option<MatchingListenerInner>,
  /// Released together with `inner` on undeclare, so the handler is dropped
  /// exactly as zenoh's own `undeclare` does.
  receiver: Option<ChannelReceiver<ZenohStatus>>,
}

impl MatchingListener {
  pub(crate) async fn declare(
    builder: MatchingListenerBuilder<'_, DefaultHandler>,
    channel: Option<ChannelHandler>,
  ) -> Result<Self> {
    let (kind, capacity) = match channel {
      Some(channel) => (channel.kind, channel.capacity),
      None => (ChannelType::Fifo, None),
    };
    let (inner, receiver) = match kind {
      ChannelType::Fifo => {
        let (handler, receiver) = handlers::fifo_parts::<ZenohStatus>(capacity);
        let listener = builder.with(handler).await.map_err(to_napi_err)?;
        (MatchingListenerInner::Fifo(listener), receiver)
      }
      ChannelType::Ring => {
        let (handler, receiver) = handlers::ring_parts::<ZenohStatus>(capacity);
        let listener = builder.with(handler).await.map_err(to_napi_err)?;
        (MatchingListenerInner::Ring(listener), receiver)
      }
    };
    Ok(Self {
      inner: Some(inner),
      receiver: Some(receiver),
    })
  }
}

#[napi]
impl MatchingListener {
  /// Wait for the next matching-status change, resolving to `null` once the
  /// listener is closed.
  #[napi]
  pub async fn recv(&self) -> Result<Option<MatchingStatus>> {
    let receiver = self.receiver.clone();
    match receiver {
      Some(receiver) => Ok(receiver.recv().await.map(MatchingStatus::from_zenoh)),
      None => Ok(None),
    }
  }

  /// Return a buffered status if one is immediately available, or `null` if the
  /// channel is currently empty. Throws once the listener has disconnected,
  /// letting a polling loop tell "nothing yet" apart from "closed".
  #[napi]
  pub fn try_recv(&self) -> Result<Option<MatchingStatus>> {
    match &self.receiver {
      Some(receiver) => receiver
        .try_recv()
        .map(|status| status.map(MatchingStatus::from_zenoh))
        .map_err(to_napi_err),
      None => Err(Error::from_reason("matching listener has been undeclared")),
    }
  }

  /// Undeclare the listener; any buffered statuses are dropped with the handler.
  /// Resolves synchronously.
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
impl AsyncGenerator for MatchingListener {
  type Yield = MatchingStatus;
  type Next = ();
  type Return = ();

  fn next(
    &mut self,
    _value: Option<Self::Next>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let receiver = self.receiver.clone();
    async move {
      match receiver {
        Some(receiver) => Ok(receiver.recv().await.map(MatchingStatus::from_zenoh)),
        None => Ok(None),
      }
    }
  }
}
