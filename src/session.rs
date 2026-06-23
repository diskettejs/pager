use napi::bindgen_prelude::{Either, Uint8Array};
use napi_derive::napi;
use zenoh::bytes::ZBytes;
use zenoh::handlers::{FifoChannel, RingChannel};
use zenoh_ext::{AdvancedPublisherBuilderExt, AdvancedSubscriberBuilderExt};

use crate::config::Config;
use crate::handlers::{ChannelKind, DEFAULT_CHANNEL_CAPACITY};
use crate::keyexpr::KeyExprArg;
use crate::options::{PublisherOptions, PutOptions, SubscriberOptions};
use crate::publisher::Publisher;
use crate::subscriber::Subscriber;

#[napi]
pub struct Session {
  inner: zenoh::Session,
}

impl Session {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: zenoh::Session) -> Self {
    Session { inner }
  }
}

#[napi]
impl Session {
  /// Opens a session with the given configuration.
  #[napi(factory)]
  pub async fn open(config: &Config) -> napi::Result<Session> {
    let cfg = config.inner.clone();
    let session = zenoh::open(cfg)
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Session::from_inner(session))
  }

  /// This session's Zenoh id, as a hex string.
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.inner.zid().to_string()
  }

  /// Whether the session has been closed.
  #[napi(getter)]
  pub fn is_closed(&self) -> bool {
    self.inner.is_closed()
  }

  /// Closes the session, undeclaring everything declared on it.
  #[napi]
  pub async fn close(&self) -> napi::Result<()> {
    let session = self.inner.clone();
    session
      .close()
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Publishes `payload` on `keyExpr`.
  #[napi]
  pub async fn put(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    payload: Either<String, Uint8Array>,
    options: Option<PutOptions>,
  ) -> napi::Result<()> {
    let session = self.inner.clone();
    let ke = key_expr.0;
    let payload: ZBytes = match payload {
      Either::A(s) => ZBytes::from(s),
      Either::B(bytes) => ZBytes::from(bytes.to_vec()),
    };

    let mut builder = session.put(ke, payload);
    if let Some(opts) = options {
      if let Some(encoding) = opts.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(congestion_control) = opts.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = opts.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = opts.express {
        builder = builder.express(express);
      }
      if let Some(reliability) = opts.reliability {
        builder = builder.reliability(reliability.into());
      }
      if let Some(allowed_destination) = opts.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      if let Some(timestamp) = opts.timestamp {
        builder = builder.timestamp(timestamp.0);
      }
      if let Some(attachment) = opts.attachment {
        builder = builder.attachment(attachment.to_vec());
      }
      if let Some(source_info) = opts.source_info {
        builder = builder.source_info(source_info.0);
      }
    }

    builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Declares a subscription on `keyExpr`.
  ///
  /// The `handler` option chooses the channel (default: FIFO of
  /// [`DEFAULT_CHANNEL_CAPACITY`]). Advanced options (history, recovery,
  /// detection, …) are wired in a later phase.
  #[napi]
  pub async fn declare_subscriber(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<SubscriberOptions>,
  ) -> napi::Result<Subscriber> {
    let session = self.inner.clone();
    let ke = key_expr.0;

    let handler_cfg = options.as_ref().and_then(|o| o.handler.as_ref());
    let capacity = handler_cfg
      .and_then(|c| c.capacity)
      .map(|c| c as usize)
      .unwrap_or(DEFAULT_CHANNEL_CAPACITY);
    let is_ring = handler_cfg.is_some_and(|c| matches!(c.kind, ChannelKind::Ring));

    // TODO(advanced-options): wire allowed_origin / history / recovery /
    // subscriber_detection / subscriber_detection_metadata / query_timeout onto
    // the advanced builder.
    if is_ring {
      let sub = session
        .declare_subscriber(ke)
        .advanced()
        .with(RingChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = sub.key_expr().clone();
      let id = sub.id();
      Ok(Subscriber::from_ring(sub, key_expr, id))
    } else {
      let sub = session
        .declare_subscriber(ke)
        .advanced()
        .with(FifoChannel::new(capacity))
        .await
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;
      let key_expr = sub.key_expr().clone();
      let id = sub.id();
      Ok(Subscriber::from_fifo(sub, key_expr, id))
    }
  }

  /// Declares a publisher on `keyExpr`, fixing its QoS for every publication.
  ///
  /// Advanced options (cache, sample-miss detection, publisher detection) are
  /// wired in a later phase.
  #[napi]
  pub async fn declare_publisher(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<PublisherOptions>,
  ) -> napi::Result<Publisher> {
    let session = self.inner.clone();
    let ke = key_expr.0;

    let mut builder = session.declare_publisher(ke).advanced();
    if let Some(opts) = options {
      if let Some(encoding) = opts.encoding {
        builder = builder.encoding(encoding);
      }
      if let Some(congestion_control) = opts.congestion_control {
        builder = builder.congestion_control(congestion_control.into());
      }
      if let Some(priority) = opts.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = opts.express {
        builder = builder.express(express);
      }
      if let Some(reliability) = opts.reliability {
        builder = builder.reliability(reliability.into());
      }
      if let Some(allowed_destination) = opts.allowed_destination {
        builder = builder.allowed_destination(allowed_destination.into());
      }
      // TODO(advanced-options): wire cache / sample_miss_detection /
      // publisher_detection / publisher_detection_metadata onto the advanced
      // builder.
    }

    let publisher = builder
      .await
      .map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Publisher::from_inner(publisher))
  }
}
