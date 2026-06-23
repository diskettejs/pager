use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannelHandler, RingChannelHandler};
use zenoh::matching::{MatchingListener as ZMatchingListener, MatchingStatus as ZMatchingStatus};

use crate::handlers::{FifoChannelHandlerMatchingStatus, RingChannelHandlerMatchingStatus};

enum ListenerInner {
  Fifo(ZMatchingListener<FifoChannelHandler<ZMatchingStatus>>),
  Ring(Arc<ZMatchingListener<RingChannelHandler<ZMatchingStatus>>>),
}

/// A listener that notifies whenever the matching status of its
/// `Publisher`/`Querier` changes (whether matching entities exist). Declared
/// via `Publisher.matchingListener`.
#[napi]
pub struct MatchingListener {
  // `None` once undeclared.
  inner: Option<ListenerInner>,
}

impl MatchingListener {
  pub(crate) fn from_fifo(
    listener: ZMatchingListener<FifoChannelHandler<ZMatchingStatus>>,
  ) -> Self {
    MatchingListener {
      inner: Some(ListenerInner::Fifo(listener)),
    }
  }

  pub(crate) fn from_ring(
    listener: ZMatchingListener<RingChannelHandler<ZMatchingStatus>>,
  ) -> Self {
    MatchingListener {
      inner: Some(ListenerInner::Ring(Arc::new(listener))),
    }
  }
}

#[napi]
impl MatchingListener {
  /// The receive end of the listener. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at declare time.
  ///
  /// The handler is not iterable; iterate via `listener.handler.stream()`.
  #[napi(getter)]
  pub fn handler(
    &self,
  ) -> napi::Result<Either<FifoChannelHandlerMatchingStatus, RingChannelHandlerMatchingStatus>> {
    match self.inner.as_ref() {
      Some(ListenerInner::Fifo(listener)) => Ok(Either::A(
        FifoChannelHandlerMatchingStatus::from_handler(listener.handler().clone()),
      )),
      Some(ListenerInner::Ring(arc)) => Ok(Either::B(RingChannelHandlerMatchingStatus::from_arc(
        Arc::clone(arc),
      ))),
      None => Err(napi::Error::from_reason(
        "matching listener has been undeclared",
      )),
    }
  }

  /// Undeclare this matching listener. Resolves once undeclaration completes; a
  /// second call is a no-op.
  ///
  /// For a ring listener still referenced by an outstanding handler, this drops
  /// our strong reference and lets the background drop undeclare it once the
  /// last handler is released.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(ListenerInner::Fifo(listener)) => listener
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      Some(ListenerInner::Ring(arc)) => match Arc::try_unwrap(arc) {
        Ok(listener) => listener
          .undeclare()
          .await
          .map_err(|e| napi::Error::from_reason(e.to_string())),
        Err(_) => Ok(()),
      },
      None => Ok(()),
    }
  }
}
