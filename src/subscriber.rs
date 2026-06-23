use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannelHandler, RingChannelHandler};
use zenoh::key_expr::KeyExpr as ZKeyExpr;
use zenoh::sample::Sample as ZSample;
use zenoh::session::EntityGlobalId as ZEntityGlobalId;
use zenoh_ext::AdvancedSubscriber;

use crate::entity_global_id::EntityGlobalId;
use crate::handlers::{FifoChannelHandlerSample, RingChannelHandlerSample};
use crate::keyexpr::KeyExpr;

enum SubInner {
  Fifo(AdvancedSubscriber<FifoChannelHandler<ZSample>>),
  Ring(Arc<AdvancedSubscriber<RingChannelHandler<ZSample>>>),
}

#[napi]
pub struct Subscriber {
  // `None` once undeclared. `key_expr`/`id` are cached so they survive it.
  inner: Option<SubInner>,
  key_expr: ZKeyExpr<'static>,
  id: ZEntityGlobalId,
}

impl Subscriber {
  pub(crate) fn from_fifo(
    sub: AdvancedSubscriber<FifoChannelHandler<ZSample>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    Subscriber {
      inner: Some(SubInner::Fifo(sub)),
      key_expr,
      id,
    }
  }

  pub(crate) fn from_ring(
    sub: AdvancedSubscriber<RingChannelHandler<ZSample>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    Subscriber {
      inner: Some(SubInner::Ring(Arc::new(sub))),
      key_expr,
      id,
    }
  }
}

#[napi]
impl Subscriber {
  /// The key expression this subscription matches.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.key_expr.clone())
  }

  /// The global id of this subscription entity.
  #[napi(getter)]
  pub fn id(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(self.id.clone())
  }

  /// The receive end of the subscription. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at declare time.
  ///
  /// The handler is not iterable; iterate via `subscriber.handler.stream()`.
  #[napi(getter)]
  pub fn handler(
    &self,
  ) -> napi::Result<Either<FifoChannelHandlerSample, RingChannelHandlerSample>> {
    match self.inner.as_ref() {
      Some(SubInner::Fifo(sub)) => Ok(Either::A(FifoChannelHandlerSample::from_handler(
        sub.handler().clone(),
      ))),
      Some(SubInner::Ring(arc)) => Ok(Either::B(RingChannelHandlerSample::from_arc(Arc::clone(
        arc,
      )))),
      None => Err(napi::Error::from_reason("subscriber has been undeclared")),
    }
  }

  /// Undeclare this subscription. Resolves once undeclaration completes; a
  /// second call is a no-op.
  ///
  /// For a ring subscription still referenced by an outstanding handler, this
  /// drops our strong reference and lets the background drop undeclare it once
  /// the last handler is released.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(SubInner::Fifo(sub)) => sub
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      Some(SubInner::Ring(arc)) => match Arc::try_unwrap(arc) {
        Ok(sub) => sub
          .undeclare()
          .await
          .map_err(|e| napi::Error::from_reason(e.to_string())),
        Err(_) => Ok(()),
      },
      None => Ok(()),
    }
  }
}
