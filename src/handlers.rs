use std::sync::Arc;

use napi_derive::napi;
use zenoh::handlers::{
  Callback, FifoChannel, FifoChannelHandler, IntoHandler, RingChannel, RingChannelHandler,
};

/// Which channel kind buffers items for a subscriber or listener.
#[napi(string_enum)]
pub enum ChannelType {
  /// Bounded FIFO queue; applies backpressure to Zenoh when full.
  Fifo,
  /// Bounded ring buffer; drops the oldest item when full (never blocks).
  Ring,
}

/// Channel handler configuration for a subscriber or listener.
#[napi(object)]
pub struct ChannelHandler {
  /// `Fifo` or `Ring`.
  pub kind: ChannelType,
  /// Channel capacity. Defaults to Zenoh's default channel size when omitted.
  pub capacity: Option<u32>,
}

/// The receiving end of either channel kind, type-erased over the choice.
///
/// Both handlers are wrapped in an `Arc` so the receiver is cloneable without
/// requiring `T: Clone` (the derived `Clone` on the zenoh handlers adds that
/// bound). Cloning is needed to move the receiver into the `'static` futures the
/// async iterator yields.
///
/// The two kinds differ in how they hold the channel: a `RingChannelHandler`
/// keeps only a weak reference (buffered items vanish as soon as the callback is
/// dropped), whereas a `FifoChannelHandler` owns the receiving half outright, so
/// its buffered items outlive the callback until the handler itself is dropped.
/// The owning entity drops this receiver on undeclare to release them.
pub(crate) enum ChannelReceiver<T> {
  Fifo(Arc<FifoChannelHandler<T>>),
  Ring(Arc<RingChannelHandler<T>>),
}

impl<T> Clone for ChannelReceiver<T> {
  fn clone(&self) -> Self {
    match self {
      ChannelReceiver::Fifo(handler) => ChannelReceiver::Fifo(Arc::clone(handler)),
      ChannelReceiver::Ring(handler) => ChannelReceiver::Ring(Arc::clone(handler)),
    }
  }
}

impl<T: Send + 'static> ChannelReceiver<T> {
  /// Await the next item, or `None` once the channel is disconnected.
  pub(crate) async fn recv(&self) -> Option<T> {
    let result = match self {
      ChannelReceiver::Fifo(handler) => handler.recv_async().await,
      ChannelReceiver::Ring(handler) => handler.recv_async().await,
    };
    result.ok()
  }

  /// Try to fetch a buffered item without blocking, preserving Zenoh's three
  /// outcomes: `Ok(Some)` when an item is ready, `Ok(None)` when the channel is
  /// empty but still connected, and `Err` once it has disconnected (all senders
  /// dropped, or the ring was deleted).
  pub(crate) fn try_recv(&self) -> zenoh::Result<Option<T>> {
    match self {
      ChannelReceiver::Fifo(handler) => handler.try_recv(),
      ChannelReceiver::Ring(handler) => handler.try_recv(),
    }
  }
}

/// The `(callback, handler)` pair to hand to a builder's `.with(..)`, paired
/// with the [`ChannelReceiver`] to keep for consuming items.
type ChannelParts<T, H> = ((Callback<T>, Arc<H>), ChannelReceiver<T>);

/// Build a FIFO channel: the `(callback, handler)` pair to hand to a builder's
/// `.with(..)`, plus the receiver to keep for consuming items.
pub(crate) fn fifo_parts<T: Send + 'static>(
  capacity: Option<u32>,
) -> ChannelParts<T, FifoChannelHandler<T>> {
  let channel = match capacity {
    Some(capacity) => FifoChannel::new(capacity as usize),
    None => FifoChannel::default(),
  };
  let (callback, handler): (Callback<T>, FifoChannelHandler<T>) = channel.into_handler();
  let handler = Arc::new(handler);
  (
    (callback, Arc::clone(&handler)),
    ChannelReceiver::Fifo(handler),
  )
}

/// Build a ring channel, mirroring [`fifo_parts`].
pub(crate) fn ring_parts<T: Send + 'static>(
  capacity: Option<u32>,
) -> ChannelParts<T, RingChannelHandler<T>> {
  let channel = match capacity {
    Some(capacity) => RingChannel::new(capacity as usize),
    None => RingChannel::default(),
  };
  let (callback, handler): (Callback<T>, RingChannelHandler<T>) = channel.into_handler();
  let handler = Arc::new(handler);
  (
    (callback, Arc::clone(&handler)),
    ChannelReceiver::Ring(handler),
  )
}
