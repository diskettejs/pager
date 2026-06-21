use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::to_napi_err;
use crate::handlers::ChannelHandler;
use crate::keyexpr::KeyExprArg;
use crate::macros::apply_options;
use crate::query::Replies;
use crate::subscriber::Subscriber;

/// Options for [`Liveliness::declareSubscriber`].
#[napi(object)]
pub struct LivelinessSubscriberOptions {
  /// When `true`, Zenoh queries the network for the currently live tokens upon
  /// declaring the subscriber, delivering each as a `Put` sample. When `false`
  /// (the default) it does not — though currently live tokens may still arrive.
  pub history: Option<bool>,
  /// Channel handler (FIFO or Ring) backing delivery. Defaults to FIFO.
  pub handler: Option<ChannelHandler>,
}

/// Options for [`Liveliness::get`].
#[napi(object)]
pub struct LivelinessGetOptions {
  /// How long to wait for replies, in milliseconds. Defaults to the session's
  /// configured query timeout.
  pub timeout: Option<u32>,
  /// Channel handler (FIFO or Ring) backing reply delivery. Defaults to FIFO.
  pub handler: Option<ChannelHandler>,
}

/// Declares liveliness tokens, queries existing ones, and subscribes to
/// liveliness changes (mirrors `zenoh`'s `Liveliness`).
///
/// A [`LivelinessToken`] is a token whose liveliness is tied to the [`Session`]
/// that declared it and can be monitored by remote applications. Obtain this
/// accessor with [`Session::liveliness`].
///
/// [`Session`]: crate::session::Session
#[napi]
pub struct Liveliness {
  // A cloned session: `zenoh::Session` is cheaply Clone (Arc-backed), and each
  // operation goes through `self.session.liveliness().<op>(..)`, mirroring how
  // `SessionInfo` holds its own clone.
  session: zenoh::Session,
}

impl Liveliness {
  pub(crate) fn new(session: zenoh::Session) -> Self {
    Self { session }
  }
}

#[napi]
impl Liveliness {
  /// Declare a [`LivelinessToken`] for `key_expr`. The token is seen as alive by
  /// any application monitoring it until it is undeclared or dropped (or this
  /// session loses connectivity / stops).
  #[napi]
  pub async fn declare_token(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
  ) -> Result<LivelinessToken> {
    let token = self
      .session
      .liveliness()
      .declare_token(key_expr.0)
      .await
      .map_err(to_napi_err)?;
    Ok(LivelinessToken::new(token))
  }

  /// Declare a [`Subscriber`] for liveliness changes matching `key_expr`. Each
  /// sample is a `Put` when a matching token appears and a `Delete` when one
  /// vanishes. Samples are delivered through a FIFO channel, consumable as an
  /// async iterator or via `recv`/`tryRecv`.
  #[napi]
  pub async fn declare_subscriber(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<LivelinessSubscriberOptions>,
  ) -> Result<Subscriber> {
    let mut builder = self.session.liveliness().declare_subscriber(key_expr.0);
    let mut channel = None;
    if let Some(options) = options {
      apply_options!(builder, options, {
        history,
      });
      channel = options.handler;
    }
    Subscriber::declare_liveliness(builder, channel).await
  }

  /// Query liveliness tokens whose key expression matches `key_expr`, receiving
  /// the matching tokens' replies through a channel, consumable as an async
  /// iterator or via `recv`/`tryRecv`.
  #[napi]
  pub async fn get(
    &self,
    #[napi(ts_arg_type = "string | KeyExpr")] key_expr: KeyExprArg,
    options: Option<LivelinessGetOptions>,
  ) -> Result<Replies> {
    let mut builder = self.session.liveliness().get(key_expr.0);
    let mut channel = None;
    if let Some(options) = options {
      apply_options!(builder, options, {
        timeout => duration_ms,
      });
      channel = options.handler;
    }
    Replies::from_liveliness_get(builder, channel).await
  }
}

/// A token whose liveliness is tied to the Zenoh [`Session`] that declared it.
///
/// While the token is not undeclared or dropped — and while the declaring
/// application is alive and has connectivity with the monitoring application —
/// any application monitoring it (via [`Liveliness::declareSubscriber`] or
/// [`Liveliness::get`]) sees it as alive. Tokens are automatically undeclared
/// when dropped. Create one with [`Liveliness::declareToken`].
///
/// [`Session`]: crate::session::Session
#[napi]
pub struct LivelinessToken {
  inner: Option<zenoh::liveliness::LivelinessToken>,
}

impl LivelinessToken {
  pub(crate) fn new(inner: zenoh::liveliness::LivelinessToken) -> Self {
    Self { inner: Some(inner) }
  }
}

#[napi]
impl LivelinessToken {
  /// Undeclare this liveliness token, so monitoring applications stop seeing it
  /// as alive. Subsequent calls are no-ops. Resolves synchronously, so awaiting
  /// the returned value is optional.
  #[napi]
  pub fn undeclare(&mut self) -> Result<()> {
    use zenoh::Wait;
    match self.inner.take() {
      Some(token) => token.undeclare().wait().map_err(to_napi_err),
      None => Ok(()),
    }
  }
}
