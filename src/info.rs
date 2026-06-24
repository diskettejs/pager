use std::sync::Arc;

use crate::handlers::{
  ChannelKind, DEFAULT_CHANNEL_CAPACITY, FifoChannelHandlerTransportEvent,
  RingChannelHandlerTransportEvent,
};
use crate::link::{Link, LinkEventsListener};
use crate::options::{LinkEventsListenerOptions, TransportEventsListenerOptions};
use crate::protocol::{Locator, WhatAmI};
use crate::sample::SampleKind;
use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannel, FifoChannelHandler, RingChannel, RingChannelHandler};
use zenoh::session::{
  Transport as ZTransport, TransportEvent as ZTransportEvent, TransportEventsListener as ZListener,
};

#[napi]
pub struct SessionInfo {
  session: zenoh::Session,
}

impl SessionInfo {
  pub(crate) fn from_session(session: zenoh::Session) -> Self {
    SessionInfo { session }
  }
}

#[napi]
impl SessionInfo {
  /// This session's Zenoh id, as a hex string.
  #[napi]
  pub async fn zid(&self) -> String {
    self.session.info().zid().await.to_string()
  }

  /// The Zenoh ids of the routers this session is currently connected to (or of
  /// the current router, if running inside one), as hex strings.
  #[napi]
  pub async fn routers_zid(&self) -> Vec<String> {
    self
      .session
      .info()
      .routers_zid()
      .await
      .map(|zid| zid.to_string())
      .collect()
  }

  /// The Zenoh ids of the peers this session is currently connected to, as hex
  /// strings.
  #[napi]
  pub async fn peers_zid(&self) -> Vec<String> {
    self
      .session
      .info()
      .peers_zid()
      .await
      .map(|zid| zid.to_string())
      .collect()
  }

  /// The locators this session is listening on.
  #[napi]
  pub async fn locators(&self) -> Vec<Locator> {
    self
      .session
      .info()
      .locators()
      .await
      .into_iter()
      .map(Locator::from_inner)
      .collect()
  }

  /// The currently-open transports (connections to remote nodes).
  #[napi]
  pub async fn transports(&self) -> Vec<Transport> {
    self
      .session
      .info()
      .transports()
      .await
      .map(Transport::from_inner)
      .collect()
  }

  /// The currently-established links across all transports.
  #[napi]
  pub async fn links(&self) -> Vec<Link> {
    self
      .session
      .info()
      .links()
      .await
      .map(Link::from_inner)
      .collect()
  }

  /// Declares a listener for transport lifecycle events (a transport opening or
  /// closing). The `handler` option chooses the channel (default: FIFO with
  /// capacity 256); `history` replays the currently-open transports on
  /// declaration.
  #[napi]
  pub async fn transport_events_listener(
    &self,
    options: Option<TransportEventsListenerOptions>,
  ) -> napi::Result<TransportEventsListener> {
    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));
    let history = options.as_ref().and_then(|o| o.history).unwrap_or(false);

    let info = self.session.info();
    let builder = info.transport_events_listener().history(history);

    if is_ring {
      let listener = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(TransportEventsListener::from_ring(listener))
    } else {
      let listener = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(TransportEventsListener::from_fifo(listener))
    }
  }

  /// Declares a listener for link lifecycle events (a link being added or
  /// removed). The `handler` option chooses the channel (default: FIFO with
  /// capacity 256); `history` replays the currently-established links on
  /// declaration.
  #[napi]
  pub async fn link_events_listener(
    &self,
    options: Option<LinkEventsListenerOptions>,
  ) -> napi::Result<LinkEventsListener> {
    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));
    let history = options.as_ref().and_then(|o| o.history).unwrap_or(false);

    let info = self.session.info();
    let builder = info.link_events_listener().history(history);

    if is_ring {
      let listener = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(LinkEventsListener::from_ring(listener))
    } else {
      let listener = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(LinkEventsListener::from_fifo(listener))
    }
  }
}

/// A transport is a connection established to a remote zenoh node.
#[napi]
pub struct Transport {
  inner: ZTransport,
}

impl Transport {
  pub(crate) fn from_inner(inner: ZTransport) -> Self {
    Transport { inner }
  }
}

#[napi]
impl Transport {
  /// The Zenoh id of the remote node, as a hex string.
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  /// The type of the remote node (Router, Peer or Client).
  #[napi(getter)]
  pub fn whatami(&self) -> WhatAmI {
    self.inner.whatami().into()
  }

  /// Whether this transport supports QoS.
  #[napi(getter)]
  pub fn is_qos(&self) -> bool {
    self.inner.is_qos()
  }

  /// Whether this transport is multicast.
  #[napi(getter)]
  pub fn is_multicast(&self) -> bool {
    self.inner.is_multicast()
  }
}

/// An event emitted when a transport is opened or closed. `kind` is `Put` when
/// the transport opened and `Delete` when it closed.
///
/// Delivered by a `TransportEventsListener`.
#[napi]
pub struct TransportEvent {
  inner: ZTransportEvent,
}

impl TransportEvent {
  pub(crate) fn from_inner(inner: ZTransportEvent) -> Self {
    TransportEvent { inner }
  }
}

#[napi]
impl TransportEvent {
  /// `Put` if the transport opened, `Delete` if it closed.
  #[napi(getter)]
  pub fn kind(&self) -> SampleKind {
    self.inner.kind().into()
  }

  /// The transport this event is about.
  #[napi(getter)]
  pub fn transport(&self) -> Transport {
    Transport::from_inner(self.inner.transport().clone())
  }
}

enum ListenerInner {
  Fifo(ZListener<FifoChannelHandler<ZTransportEvent>>),
  Ring(Arc<ZListener<RingChannelHandler<ZTransportEvent>>>),
}

/// A listener that notifies of transport lifecycle events (a transport opening
/// or closing). Declared via `SessionInfo.transportEventsListener`.
#[napi]
pub struct TransportEventsListener {
  // `None` once undeclared.
  inner: Option<ListenerInner>,
}

impl TransportEventsListener {
  pub(crate) fn from_fifo(listener: ZListener<FifoChannelHandler<ZTransportEvent>>) -> Self {
    TransportEventsListener {
      inner: Some(ListenerInner::Fifo(listener)),
    }
  }

  pub(crate) fn from_ring(listener: ZListener<RingChannelHandler<ZTransportEvent>>) -> Self {
    TransportEventsListener {
      inner: Some(ListenerInner::Ring(Arc::new(listener))),
    }
  }
}

#[napi]
impl TransportEventsListener {
  /// The receive end of the listener. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at declare time.
  #[napi(getter)]
  pub fn handler(
    &self,
  ) -> napi::Result<Either<FifoChannelHandlerTransportEvent, RingChannelHandlerTransportEvent>> {
    match self.inner.as_ref() {
      Some(ListenerInner::Fifo(listener)) => Ok(Either::A(
        FifoChannelHandlerTransportEvent::from_handler(listener.handler().clone()),
      )),
      Some(ListenerInner::Ring(arc)) => Ok(Either::B(RingChannelHandlerTransportEvent::from_arc(
        Arc::clone(arc),
      ))),
      None => Err(napi::Error::from_reason(
        "transport events listener has been undeclared",
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
