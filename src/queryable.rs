use std::sync::Arc;

use napi::bindgen_prelude::Either;
use napi_derive::napi;
use zenoh::handlers::{FifoChannelHandler, RingChannelHandler};
use zenoh::key_expr::KeyExpr as ZKeyExpr;
use zenoh::query::{Query as ZQuery, Queryable as ZQueryable};
use zenoh::session::EntityGlobalId as ZEntityGlobalId;

use crate::handlers::{FifoChannelHandlerQuery, RingChannelHandlerQuery};
use crate::keyexpr::KeyExpr;
use crate::session::EntityGlobalId;

enum QueryableInner {
  Fifo(ZQueryable<FifoChannelHandler<ZQuery>>),
  Ring(Arc<ZQueryable<RingChannelHandler<ZQuery>>>),
}

#[napi]
pub struct Queryable {
  // `None` once undeclared. `key_expr`/`id` are cached so they survive it.
  inner: Option<QueryableInner>,
  key_expr: ZKeyExpr<'static>,
  id: ZEntityGlobalId,
}

impl Queryable {
  pub(crate) fn from_fifo(
    queryable: ZQueryable<FifoChannelHandler<ZQuery>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    Queryable {
      inner: Some(QueryableInner::Fifo(queryable)),
      key_expr,
      id,
    }
  }

  pub(crate) fn from_ring(
    queryable: ZQueryable<RingChannelHandler<ZQuery>>,
    key_expr: ZKeyExpr<'static>,
    id: ZEntityGlobalId,
  ) -> Self {
    Queryable {
      inner: Some(QueryableInner::Ring(Arc::new(queryable))),
      key_expr,
      id,
    }
  }
}

#[napi]
impl Queryable {
  /// The key expression this queryable answers queries on.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.key_expr.clone())
  }

  /// The global id of this queryable entity.
  #[napi(getter)]
  pub fn id(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(self.id)
  }

  /// The receive end delivering incoming queries. A `FifoChannelHandler` or
  /// `RingChannelHandler` depending on the channel chosen at declare time.
  #[napi(getter)]
  pub fn handler(&self) -> napi::Result<Either<FifoChannelHandlerQuery, RingChannelHandlerQuery>> {
    match self.inner.as_ref() {
      Some(QueryableInner::Fifo(queryable)) => Ok(Either::A(
        FifoChannelHandlerQuery::from_handler(queryable.handler().clone()),
      )),
      Some(QueryableInner::Ring(arc)) => Ok(Either::B(RingChannelHandlerQuery::from_arc(
        Arc::clone(arc),
      ))),
      None => Err(napi::Error::from_reason("queryable has been undeclared")),
    }
  }

  /// Undeclare this queryable. Resolves once undeclaration completes; a second
  /// call is a no-op.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(QueryableInner::Fifo(queryable)) => queryable
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      Some(QueryableInner::Ring(arc)) => match Arc::try_unwrap(arc) {
        Ok(queryable) => queryable
          .undeclare()
          .await
          .map_err(|e| napi::Error::from_reason(e.to_string())),
        Err(_) => Ok(()),
      },
      None => Ok(()),
    }
  }
}
