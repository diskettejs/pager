use napi::bindgen_prelude::{Either, Uint8Array};
use napi_derive::napi;
use zenoh::bytes::Encoding as ZEncoding;
use zenoh::bytes::ZBytes;
use zenoh::handlers::{FifoChannel, RingChannel};
use zenoh::key_expr::KeyExpr as ZKeyExpr;
use zenoh::qos::{CongestionControl as ZCongestionControl, Priority as ZPriority};
use zenoh::session::EntityGlobalId as ZEntityGlobalId;
use zenoh_ext::AdvancedPublisher;

use crate::encoding::Encoding;
use crate::handlers::{ChannelKind, DEFAULT_CHANNEL_CAPACITY};
use crate::keyexpr::KeyExpr;
use crate::matching::{MatchingListener, MatchingStatus};
use crate::options::{MatchingListenerOptions, PublisherDeleteOptions, PublisherPutOptions};
use crate::qos::{CongestionControl, Priority};
use crate::session::EntityGlobalId;

#[napi]
pub struct Publisher {
  // `None` once undeclared. The cached config below survives it.
  inner: Option<AdvancedPublisher<'static>>,
  key_expr: ZKeyExpr<'static>,
  id: ZEntityGlobalId,
  encoding: ZEncoding,
  congestion_control: ZCongestionControl,
  priority: ZPriority,
}

impl Publisher {
  /// Internal constructor: cache the publisher's fixed config, then take
  /// ownership of it.
  pub(crate) fn from_inner(publisher: AdvancedPublisher<'static>) -> Self {
    let key_expr = publisher.key_expr().clone();
    let id = publisher.id();
    let encoding = publisher.encoding().clone();
    let congestion_control = publisher.congestion_control();
    let priority = publisher.priority();
    Publisher {
      inner: Some(publisher),
      key_expr,
      id,
      encoding,
      congestion_control,
      priority,
    }
  }
}

#[napi]
impl Publisher {
  /// The key expression this publisher publishes on.
  #[napi(getter)]
  pub fn key_expr(&self) -> KeyExpr {
    KeyExpr::from_inner(self.key_expr.clone())
  }

  /// The global id of this publisher entity.
  #[napi(getter)]
  pub fn id(&self) -> EntityGlobalId {
    EntityGlobalId::from_inner(self.id)
  }

  /// The encoding applied to data published by this publisher.
  #[napi(getter)]
  pub fn encoding(&self) -> Encoding {
    Encoding::from_inner(self.encoding.clone())
  }

  /// The congestion control applied when routing this publisher's data.
  #[napi(getter)]
  pub fn congestion_control(&self) -> CongestionControl {
    self.congestion_control.into()
  }

  /// The priority of data published by this publisher.
  #[napi(getter)]
  pub fn priority(&self) -> Priority {
    self.priority.into()
  }

  /// Publishes `payload` on this publisher's key expression.
  #[napi]
  pub async fn put(
    &self,
    payload: Either<String, Uint8Array>,
    options: Option<PublisherPutOptions>,
  ) -> napi::Result<()> {
    let publisher = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("publisher has been undeclared"))?;
    let payload: ZBytes = match payload {
      Either::A(s) => ZBytes::from(s),
      Either::B(bytes) => ZBytes::from(bytes.to_vec()),
    };

    let mut builder = publisher.put(payload);
    if let Some(opts) = options {
      if let Some(encoding) = opts.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(timestamp) = opts.timestamp {
        builder = builder.timestamp(timestamp.0);
      }
      if let Some(attachment) = opts.attachment {
        builder = builder.attachment(attachment.to_vec());
      }
    }

    builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Publishes a delete on this publisher's key expression.
  #[napi]
  pub async fn delete(&self, options: Option<PublisherDeleteOptions>) -> napi::Result<()> {
    let publisher = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("publisher has been undeclared"))?;

    let mut builder = publisher.delete();
    if let Some(opts) = options {
      if let Some(timestamp) = opts.timestamp {
        builder = builder.timestamp(timestamp.0);
      }
      if let Some(attachment) = opts.attachment {
        builder = builder.attachment(attachment.to_vec());
      }
    }

    builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// The current matching status of this publisher — whether any subscribers
  /// match its key expression.
  #[napi]
  pub async fn matching_status(&self) -> napi::Result<MatchingStatus> {
    let publisher = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("publisher has been undeclared"))?;
    let status = publisher
      .matching_status()
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(MatchingStatus::from_inner(status))
  }

  /// Declares a listener that notifies whenever this publisher's matching status
  /// changes (subscribers appear or disappear).
  #[napi]
  pub async fn matching_listener(
    &self,
    options: Option<MatchingListenerOptions>,
  ) -> napi::Result<MatchingListener> {
    let publisher = self
      .inner
      .as_ref()
      .ok_or_else(|| napi::Error::from_reason("publisher has been undeclared"))?;

    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    if is_ring {
      let listener = publisher
        .matching_listener()
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(MatchingListener::from_ring(listener))
    } else {
      let listener = publisher
        .matching_listener()
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      Ok(MatchingListener::from_fifo(listener))
    }
  }

  /// Undeclare this publisher. Resolves once undeclaration completes; a second
  /// call is a no-op.
  #[napi]
  pub async unsafe fn undeclare(&mut self) -> napi::Result<()> {
    match self.inner.take() {
      Some(publisher) => publisher
        .undeclare()
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string())),
      None => Ok(()),
    }
  }
}
