use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannelHandler, RingChannelHandler};
use zenoh_ext::{Miss as ZMiss, SampleMissListener as ZSampleMissListener};

use crate::handlers::{FifoChannelHandlerMiss, RingChannelHandlerMiss};

enum ListenerInner {
  Fifo(ZSampleMissListener<FifoChannelHandler<ZMiss>>),
  Ring(Arc<ZSampleMissListener<RingChannelHandler<ZMiss>>>),
}

/// A listener that notifies of missed samples on a subscription. Declared via
/// `Subscriber.sampleMissListener`; misses are only detected when the matching
/// publisher enables `sampleMissDetection`.
#[napi]
pub struct SampleMissListener {
  // `None` once undeclared.
  inner: Option<ListenerInner>,
}

impl SampleMissListener {
  pub(crate) fn from_fifo(listener: ZSampleMissListener<FifoChannelHandler<ZMiss>>) -> Self {
    SampleMissListener {
      inner: Some(ListenerInner::Fifo(listener)),
    }
  }

  pub(crate) fn from_ring(listener: ZSampleMissListener<RingChannelHandler<ZMiss>>) -> Self {
    SampleMissListener {
      inner: Some(ListenerInner::Ring(Arc::new(listener))),
    }
  }
}

#[napi]
impl SampleMissListener {
  /// The receive end of the listener. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at declare time.
  ///
  /// The handler is not iterable; iterate via `listener.handler.stream()`.
  #[napi(getter)]
  pub fn handler(&self) -> napi::Result<Either<FifoChannelHandlerMiss, RingChannelHandlerMiss>> {
    match self.inner.as_ref() {
      // `SampleMissListener` has no inherent `.handler()`; it `Deref`s to its
      // handler, so coerce `&listener` to `&FifoChannelHandler<_>` and clone.
      Some(ListenerInner::Fifo(listener)) => {
        let handler: &FifoChannelHandler<ZMiss> = listener;
        Ok(Either::A(FifoChannelHandlerMiss::from_handler(
          handler.clone(),
        )))
      }
      Some(ListenerInner::Ring(arc)) => {
        Ok(Either::B(RingChannelHandlerMiss::from_arc(Arc::clone(arc))))
      }
      None => Err(napi::Error::from_reason(
        "sample miss listener has been undeclared",
      )),
    }
  }

  /// Undeclare this sample-miss listener. Resolves once undeclaration completes;
  /// a second call is a no-op.
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
