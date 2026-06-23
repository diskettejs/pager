//! `Scout` — discover other zenoh processes (routers/peers) on the network,
//! independently of any session.
//!
//! `Scout.scout(what, config)` spawns a background task that periodically
//! multicasts scout messages and delivers each `Hello` reply through the chosen
//! channel. Modeled as a factory on `Scout` (like `Session.open`), since it is
//! the sole constructor of a `Scout`. The returned handle keeps scouting until
//! `stop` is called or it is dropped. Unlike the session entities, cleanup is
//! synchronous (a local task cancel, no network round-trip), so `Scout` is
//! `Disposable` (`using`), not async.

use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannel, FifoChannelHandler, RingChannel, RingChannelHandler};
use zenoh::scouting::{Hello as ZHello, Scout as ZScout};

use crate::config::Config;
use crate::handlers::{
  ChannelKind, DEFAULT_CHANNEL_CAPACITY, FifoChannelHandlerHello, RingChannelHandlerHello,
};
use crate::options::ScoutOptions;
use crate::whatami_matcher::WhatAmIMatcher;

enum ScoutInner {
  Fifo(ZScout<FifoChannelHandler<ZHello>>),
  Ring(Arc<ZScout<RingChannelHandler<ZHello>>>),
}

#[napi]
pub struct Scout {
  // `None` once stopped. A `Scout` has no key expression or id — only a handler.
  inner: Option<ScoutInner>,
}

impl Scout {
  fn from_fifo(scout: ZScout<FifoChannelHandler<ZHello>>) -> Self {
    Scout {
      inner: Some(ScoutInner::Fifo(scout)),
    }
  }

  fn from_ring(scout: ZScout<RingChannelHandler<ZHello>>) -> Self {
    Scout {
      inner: Some(ScoutInner::Ring(Arc::new(scout))),
    }
  }
}

#[napi]
impl Scout {
  /// Scout for zenoh processes matching `what` (router/peer/client), using
  /// `config` for the multicast settings.
  ///
  /// The `handler` option chooses the channel delivering `Hello` replies
  /// (default: FIFO of [`DEFAULT_CHANNEL_CAPACITY`]). The returned `Scout` keeps
  /// scouting until `stop` is called or it is dropped.
  #[napi(factory)]
  pub async fn scout(
    what: &WhatAmIMatcher,
    config: &Config,
    options: Option<ScoutOptions>,
  ) -> napi::Result<Scout> {
    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    let what = what.inner;
    let config = config.inner.clone();
    let builder = zenoh::scout(what, config);

    if is_ring {
      let scout = builder
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(Scout::from_ring(scout))
    } else {
      let scout = builder
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(Scout::from_fifo(scout))
    }
  }

  /// The receive end delivering `Hello` replies. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at scout time.
  ///
  /// The handler is not iterable; iterate via `scout.handler.stream()`.
  #[napi(getter)]
  pub fn handler(&self) -> napi::Result<Either<FifoChannelHandlerHello, RingChannelHandlerHello>> {
    match self.inner.as_ref() {
      // `Scout` derefs to its receiver, so coerce `&Scout` to `&handler`.
      Some(ScoutInner::Fifo(scout)) => {
        let handler: &FifoChannelHandler<ZHello> = scout;
        Ok(Either::A(FifoChannelHandlerHello::from_handler(
          handler.clone(),
        )))
      }
      Some(ScoutInner::Ring(arc)) => Ok(Either::B(RingChannelHandlerHello::from_arc(Arc::clone(
        arc,
      )))),
      None => Err(napi::Error::from_reason("scout has been stopped")),
    }
  }

  /// Stop scouting. Idempotent; a second call is a no-op.
  ///
  /// For a ring scout still referenced by an outstanding handler, this drops our
  /// strong reference and lets the last handler release stop it. Dropping the
  /// `Scout` does the same.
  #[napi]
  pub unsafe fn stop(&mut self) {
    // Taking the inner out and dropping it cancels the scouting task (its `Drop`
    // calls `stop`); for a ring scout the task lives until the last handle goes.
    let _ = self.inner.take();
  }
}
