use crate::error::zerr;
use crate::sample::{Sample, to_js_sample};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::future::Future;

type SampleSubscriber =
  zenoh::pubsub::Subscriber<zenoh::handlers::FifoChannelHandler<zenoh::sample::Sample>>;

/// Channel-mode subscriber: an async iterator over Zenoh's FIFO handler.
#[napi(async_iterator)]
pub struct Subscriber {
  key_expr: String,
  // Kept alive so the subscription stays declared; behind a Mutex so `undeclare`
  // (and the iterator's `complete`) can take it through a shared `&self`.
  subscriber: std::sync::Mutex<Option<SampleSubscriber>>,
  handler: zenoh::handlers::FifoChannelHandler<zenoh::sample::Sample>,
}

impl Subscriber {
  pub(crate) fn new(subscriber: SampleSubscriber) -> Self {
    let key_expr = subscriber.key_expr().to_string();
    // Clone the cheap FIFO receiver for use inside `'static` next() futures.
    let handler = subscriber.handler().clone();
    Self {
      key_expr,
      subscriber: std::sync::Mutex::new(Some(subscriber)),
      handler,
    }
  }
}

#[napi]
impl Subscriber {
  #[napi(getter)]
  pub fn key_expr(&self) -> String {
    self.key_expr.clone()
  }

  /// Pull one sample, or `null` once the subscription is closed/undeclared.
  #[napi]
  pub async fn receive(&self) -> Result<Option<Sample>> {
    let handler = self.handler.clone();
    match handler.recv_async().await {
      Ok(sample) => Ok(Some(to_js_sample(sample))),
      Err(_) => Ok(None),
    }
  }

  #[napi]
  pub async fn undeclare(&self) -> Result<()> {
    let taken = self.subscriber.lock().unwrap().take();
    if let Some(s) = taken {
      s.undeclare()
        .await
        .map_err(|e| zerr("subscriber.undeclare", e))?;
    }
    Ok(())
  }
}

/// `next()` returns a `'static` future, so it cannot borrow `self` — we clone the
/// cheap FIFO receiver and move it in.
#[napi]
impl AsyncGenerator for Subscriber {
  type Yield = Sample;
  type Next = ();
  type Return = ();

  fn next(
    &mut self,
    _value: Option<Self::Next>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let handler = self.handler.clone();
    async move {
      // Ok(sample) -> yield it; Err means all senders dropped -> iteration ends.
      match handler.recv_async().await {
        Ok(sample) => Ok(Some(to_js_sample(sample))),
        Err(_) => Ok(None),
      }
    }
  }

  /// Called when the consumer `break`s out of `for await` (AsyncGenerator.return()):
  /// undeclare the subscription for clean teardown.
  fn complete(
    &mut self,
    _value: Option<Self::Return>,
  ) -> impl Future<Output = Result<Option<Self::Yield>>> + Send + 'static {
    let subscriber = self.subscriber.lock().unwrap().take();
    async move {
      if let Some(s) = subscriber {
        let _ = s.undeclare().await;
      }
      Ok(None)
    }
  }
}
