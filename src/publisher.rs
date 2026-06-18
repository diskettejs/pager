use crate::enums::{CongestionControl, Locality, Priority};
use crate::error::zerr;
use crate::payload::to_zbytes;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

/// QoS + addressing fixed on the publisher at declare time.
#[napi(object)]
pub struct PublisherOptions {
  pub encoding: Option<String>,
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub allowed_destination: Option<Locality>,
}

/// Per-message overrides for `publisher.put()`. QoS is fixed at declare time, so
/// only content/metadata vary here. (`timestamp` override lands in a later slice —
/// it needs construction of a Zenoh `Timestamp` from JS.)
#[napi(object)]
pub struct PublisherPutOptions {
  pub encoding: Option<String>,
  pub attachment: Option<Either<String, Uint8Array>>,
}

/// Per-message overrides for `publisher.delete()`.
#[napi(object)]
pub struct PublisherDeleteOptions {
  pub attachment: Option<Either<String, Uint8Array>>,
}

/// A declared publisher. The underlying `Publisher<'static>` is kept behind an
/// `Arc` so `put`/`delete` can borrow it across the await on a shared `&self`,
/// and behind a `Mutex<Option<…>>` so `undeclare` can take ownership.
#[napi]
pub struct Publisher {
  key_expr: String,
  publisher: Mutex<Option<Arc<zenoh::pubsub::Publisher<'static>>>>,
}

impl Publisher {
  pub(crate) fn new(key_expr: String, publisher: zenoh::pubsub::Publisher<'static>) -> Self {
    Self {
      key_expr,
      publisher: Mutex::new(Some(Arc::new(publisher))),
    }
  }

  fn handle(&self, op: &str) -> Result<Arc<zenoh::pubsub::Publisher<'static>>> {
    self
      .publisher
      .lock()
      .unwrap()
      .as_ref()
      .cloned()
      .ok_or_else(|| zerr(op, "publisher is undeclared"))
  }
}

#[napi]
impl Publisher {
  #[napi(getter)]
  pub fn key_expr(&self) -> String {
    self.key_expr.clone()
  }

  #[napi]
  pub async fn put(
    &self,
    payload: Either<String, Uint8Array>,
    options: Option<PublisherPutOptions>,
  ) -> Result<()> {
    let publisher = self.handle("publisher.put")?;
    let mut builder = publisher.put(to_zbytes(payload));
    if let Some(o) = options {
      if let Some(encoding) = o.encoding {
        builder = builder.encoding(encoding.as_str());
      }
      if let Some(attachment) = o.attachment {
        builder = builder.attachment(to_zbytes(attachment));
      }
    }
    builder.await.map_err(|e| zerr("publisher.put", e))?;
    Ok(())
  }

  #[napi]
  pub async fn delete(&self, options: Option<PublisherDeleteOptions>) -> Result<()> {
    let publisher = self.handle("publisher.delete")?;
    let mut builder = publisher.delete();
    if let Some(o) = options
      && let Some(attachment) = o.attachment
    {
      builder = builder.attachment(to_zbytes(attachment));
    }
    builder.await.map_err(|e| zerr("publisher.delete", e))?;
    Ok(())
  }

  #[napi]
  pub async fn undeclare(&self) -> Result<()> {
    let taken = self.publisher.lock().unwrap().take();
    // If an in-flight put still holds an Arc clone, `try_unwrap` returns Err; we
    // just drop ours and let Zenoh undeclare when the last reference drops.
    if let Some(arc) = taken
      && let Ok(publisher) = Arc::try_unwrap(arc)
    {
      publisher
        .undeclare()
        .await
        .map_err(|e| zerr("publisher.undeclare", e))?;
    }
    Ok(())
  }
}
