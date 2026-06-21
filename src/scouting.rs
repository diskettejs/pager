use std::future::Future;
use std::sync::Arc;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use zenoh::config::{WhatAmI as ZWhatAmI, WhatAmIMatcher};
use zenoh::handlers::{FifoChannelHandler, RingChannelHandler};

use crate::config::Config;
use crate::error::to_napi_err;
use crate::handlers::{self, ChannelHandler, ChannelReceiver, ChannelType};

type FifoScout = zenoh::scouting::Scout<Arc<FifoChannelHandler<zenoh::scouting::Hello>>>;
type RingScout = zenoh::scouting::Scout<Arc<RingChannelHandler<zenoh::scouting::Hello>>>;

/// The kind of node a Zenoh process runs as (mirrors `zenoh::config::WhatAmI`).
///
/// Used both to describe a discovered node (in a [`Hello`]) and to select which
/// kinds to scout for (in [`scout`]).
#[napi(string_enum)]
pub enum WhatAmI {
  /// A router: maintains a statically-configured network topology and forwards
  /// between nodes.
  Router,
  /// A peer: discovers and connects to other nodes directly (the default mode).
  Peer,
  /// A client: stays connected to a single gateway node.
  Client,
}

impl From<ZWhatAmI> for WhatAmI {
  fn from(value: ZWhatAmI) -> Self {
    match value {
      ZWhatAmI::Router => WhatAmI::Router,
      ZWhatAmI::Peer => WhatAmI::Peer,
      ZWhatAmI::Client => WhatAmI::Client,
    }
  }
}

impl From<WhatAmI> for ZWhatAmI {
  fn from(value: WhatAmI) -> Self {
    match value {
      WhatAmI::Router => ZWhatAmI::Router,
      WhatAmI::Peer => ZWhatAmI::Peer,
      WhatAmI::Client => ZWhatAmI::Client,
    }
  }
}

/// A `Hello` message received while scouting: a discovered node's identity, kind,
/// and where it can be reached.
///
/// Fields are exposed as lazy getters, mirroring `zenoh::scouting::Hello`.
#[napi]
pub struct Hello {
  inner: zenoh::scouting::Hello,
}

impl Hello {
  pub(crate) fn new(inner: zenoh::scouting::Hello) -> Self {
    Self { inner }
  }
}

#[napi]
impl Hello {
  /// The kind of node that sent this `Hello`.
  #[napi(getter)]
  pub fn whatami(&self) -> WhatAmI {
    self.inner.whatami().into()
  }

  /// The Zenoh ID of the node, as a hex string.
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  /// The locators at which the node can be reached.
  #[napi(getter)]
  pub fn locators(&self) -> Vec<String> {
    self
      .inner
      .locators()
      .iter()
      .map(|locator| locator.to_string())
      .collect()
  }
}

/// The running scout, kept alive (and stoppable) regardless of which channel
/// kind backs it.
enum ScoutInner {
  Fifo(FifoScout),
  Ring(RingScout),
}

impl ScoutInner {
  fn stop(self) {
    match self {
      ScoutInner::Fifo(scout) => scout.stop(),
      ScoutInner::Ring(scout) => scout.stop(),
    }
  }
}

/// A scout that delivers [`Hello`] messages through a channel.
///
/// Consume it with `for await (const hello of scout)`, or pull messages
/// individually with `recv()` / `tryRecv()`. Iteration ends (yields `null`)
/// once the scout is stopped — its buffered messages are dropped with the
/// handler, as in zenoh.
#[napi(async_iterator)]
pub struct Scout {
  inner: Option<ScoutInner>,
  /// Released together with `inner` on stop, so the handler (and any messages
  /// still buffered in it) is dropped exactly as zenoh's own `stop` does,
  /// rather than left draining after the scout is gone.
  receiver: Option<ChannelReceiver<zenoh::scouting::Hello>>,
}

#[napi]
impl Scout {
  /// Wait for the next `Hello`, resolving to `null` once the scout is stopped.
  #[napi]
  pub async fn recv(&self) -> Result<Option<Hello>> {
    let receiver = self.receiver.clone();
    match receiver {
      Some(receiver) => Ok(receiver.recv().await.map(Hello::new)),
      None => Ok(None),
    }
  }

  /// Return a buffered `Hello` if one is immediately available, or `null` if the
  /// channel is currently empty. Throws once the scout has been stopped, letting
  /// a polling loop tell "nothing yet" apart from "stopped".
  #[napi]
  pub fn try_recv(&self) -> Result<Option<Hello>> {
    match &self.receiver {
      Some(receiver) => receiver
        .try_recv()
        .map(|hello| hello.map(Hello::new))
        .map_err(to_napi_err),
      None => Err(Error::from_reason("scout has been stopped")),
    }
  }

  /// Stop scouting. Iteration / `recv` then end and `tryRecv` throws; any
  /// buffered messages are dropped with the handler. Resolves synchronously.
  #[napi]
  pub fn stop(&mut self) {
    // Release the receiver with the scout: zenoh drops the handler (and anything
    // still buffered in it) as part of stopping, so mirror that instead of
    // leaving a FIFO buffer draining after the scout is gone.
    self.receiver = None;
    if let Some(inner) = self.inner.take() {
      inner.stop();
    }
  }
}

#[napi]
impl AsyncGenerator for Scout {
  type Yield = Hello;
  type Next = ();
  type Return = ();

  fn next(
    &mut self,
    _value: Option<Self::Next>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let receiver = self.receiver.clone();
    async move {
      match receiver {
        Some(receiver) => Ok(receiver.recv().await.map(Hello::new)),
        None => Ok(None),
      }
    }
  }
}

/// Scout for Zenoh nodes in the network.
///
/// Spawns a task that periodically sends scout messages and delivers the
/// [`Hello`] replies through the returned [`Scout`] handle's channel. Stop it
/// with `Scout.stop()`.
///
/// # Arguments
///
/// * `what` - The kinds of node to scout for, as [`WhatAmI`] values that are
///   folded together. An empty array matches all kinds.
/// * `config` - The [`Config`] to use for scouting.
/// * `handler` - Optional channel handler (FIFO or Ring) backing delivery.
///   Defaults to FIFO.
#[napi]
// Reachable only through napi's load-time `ctor!` registration glue, which
// rust-analyzer's dead-code analysis doesn't trace (a cdylib has no Rust-`pub`
// exports); `cargo check` sees it as used. Silences the RA false positive.
#[allow(dead_code)]
pub async fn scout(
  what: Vec<WhatAmI>,
  config: &Config,
  handler: Option<ChannelHandler>,
) -> Result<Scout> {
  let matcher = if what.is_empty() {
    // Mirror zenoh's "match everything" default rather than scouting for nothing.
    WhatAmIMatcher::empty().router().peer().client()
  } else {
    what
      .into_iter()
      .fold(WhatAmIMatcher::empty(), |matcher, kind| match kind {
        WhatAmI::Router => matcher.router(),
        WhatAmI::Peer => matcher.peer(),
        WhatAmI::Client => matcher.client(),
      })
  };

  // `zenoh::scout` consumes the config; clone the inner so a borrowed `&Config`
  // stays reusable, matching how other entry points take `&Config`.
  let config = config.inner.clone();

  let (kind, capacity) = match handler {
    Some(handler) => (handler.kind, handler.capacity),
    None => (ChannelType::Fifo, None),
  };
  let (inner, receiver) = match kind {
    ChannelType::Fifo => {
      let (handler, receiver) = handlers::fifo_parts::<zenoh::scouting::Hello>(capacity);
      let scout = zenoh::scout(matcher, config)
        .with(handler)
        .await
        .map_err(to_napi_err)?;
      (ScoutInner::Fifo(scout), receiver)
    }
    ChannelType::Ring => {
      let (handler, receiver) = handlers::ring_parts::<zenoh::scouting::Hello>(capacity);
      let scout = zenoh::scout(matcher, config)
        .with(handler)
        .await
        .map_err(to_napi_err)?;
      (ScoutInner::Ring(scout), receiver)
    }
  };

  Ok(Scout {
    inner: Some(inner),
    receiver: Some(receiver),
  })
}
