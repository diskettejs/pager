use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannelHandler, RingChannelHandler};
use zenoh::session::{Link as ZLink, LinkEvent as ZLinkEvent, LinkEventsListener as ZListener};

use crate::handlers::{FifoChannelHandlerLinkEvent, RingChannelHandlerLinkEvent};
use crate::protocol::Locator;
use crate::qos::Reliability;
use crate::sample::SampleKind;

/// The priority range `(min, max)` a link is configured with. The numeric
/// priority values correspond to `Priority` but may also
/// include `0` (Control), which is not exposed in that enum.
#[napi(object)]
pub struct LinkPriorities {
  pub min: u8,
  pub max: u8,
}

/// A concrete link within a `Transport`. Zenoh can
/// establish multiple links to the same remote node using different protocols
/// (TCP, UDP, QUIC, ...).
///
/// Obtained from `SessionInfo.links` or a `LinkEvent`.
#[napi]
pub struct Link {
  inner: ZLink,
}

impl Link {
  pub(crate) fn from_inner(inner: ZLink) -> Self {
    Link { inner }
  }
}

#[napi]
impl Link {
  /// The Zenoh id of the transport this link belongs to, as a hex string.
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  /// The source locator (local endpoint).
  #[napi(getter)]
  pub fn src(&self) -> Locator {
    Locator::from_inner(self.inner.src().clone())
  }

  /// The destination locator (remote endpoint).
  #[napi(getter)]
  pub fn dst(&self) -> Locator {
    Locator::from_inner(self.inner.dst().clone())
  }

  /// The group locator (the destination, when the link is multicast), or `null`.
  #[napi(getter)]
  pub fn group(&self) -> Option<Locator> {
    self.inner.group().cloned().map(Locator::from_inner)
  }

  /// The maximum transmission unit of the link, in bytes.
  #[napi(getter)]
  pub fn mtu(&self) -> u16 {
    self.inner.mtu()
  }

  /// Whether the link is streamed.
  #[napi(getter)]
  pub fn is_streamed(&self) -> bool {
    self.inner.is_streamed()
  }

  /// The network interfaces associated with the link.
  #[napi(getter)]
  pub fn interfaces(&self) -> Vec<String> {
    self.inner.interfaces().to_vec()
  }

  /// The authentication identifier used for the link, or `null` if none.
  #[napi(getter)]
  pub fn auth_identifier(&self) -> Option<String> {
    self.inner.auth_identifier().map(|s| s.to_string())
  }

  /// The priority range `{ min, max }` of the link, or `null` if the transport
  /// does not support QoS.
  #[napi(getter)]
  pub fn priorities(&self) -> Option<LinkPriorities> {
    self
      .inner
      .priorities()
      .map(|(min, max)| LinkPriorities { min, max })
  }

  /// The reliability level of the link, or `null` if the transport does not
  /// support QoS.
  #[napi(getter)]
  pub fn reliability(&self) -> Option<Reliability> {
    self.inner.reliability().map(Into::into)
  }
}

/// An event emitted when a link is added or removed. `kind` is `Put` when the
/// link was added and `Delete` when it was removed.
///
/// Delivered by a `LinkEventsListener`.
#[napi]
pub struct LinkEvent {
  inner: ZLinkEvent,
}

impl LinkEvent {
  pub(crate) fn from_inner(inner: ZLinkEvent) -> Self {
    LinkEvent { inner }
  }
}

#[napi]
impl LinkEvent {
  /// `Put` if the link was added, `Delete` if it was removed.
  #[napi(getter)]
  pub fn kind(&self) -> SampleKind {
    self.inner.kind().into()
  }

  /// The link this event is about.
  #[napi(getter)]
  pub fn link(&self) -> Link {
    Link::from_inner(self.inner.link().clone())
  }
}

enum ListenerInner {
  Fifo(ZListener<FifoChannelHandler<ZLinkEvent>>),
  Ring(Arc<ZListener<RingChannelHandler<ZLinkEvent>>>),
}

/// A listener that notifies of link lifecycle events (a link being added or
/// removed). Declared via `SessionInfo.linkEventsListener`.
#[napi]
pub struct LinkEventsListener {
  // `None` once undeclared.
  inner: Option<ListenerInner>,
}

impl LinkEventsListener {
  pub(crate) fn from_fifo(listener: ZListener<FifoChannelHandler<ZLinkEvent>>) -> Self {
    LinkEventsListener {
      inner: Some(ListenerInner::Fifo(listener)),
    }
  }

  pub(crate) fn from_ring(listener: ZListener<RingChannelHandler<ZLinkEvent>>) -> Self {
    LinkEventsListener {
      inner: Some(ListenerInner::Ring(Arc::new(listener))),
    }
  }
}

#[napi]
impl LinkEventsListener {
  /// The receive end of the listener. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at declare time.
  #[napi(getter)]
  pub fn handler(
    &self,
  ) -> napi::Result<Either<FifoChannelHandlerLinkEvent, RingChannelHandlerLinkEvent>> {
    match self.inner.as_ref() {
      Some(ListenerInner::Fifo(listener)) => Ok(Either::A(
        FifoChannelHandlerLinkEvent::from_handler(listener.handler().clone()),
      )),
      Some(ListenerInner::Ring(arc)) => Ok(Either::B(RingChannelHandlerLinkEvent::from_arc(
        Arc::clone(arc),
      ))),
      None => Err(napi::Error::from_reason(
        "link events listener has been undeclared",
      )),
    }
  }

  /// Undeclare this listener. Resolves once undeclaration completes; a second
  /// call is a no-op.
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
