use crate::config::Config;
use crate::enums::{CongestionControl, Locality, Priority};
use crate::error::zerr;
use crate::payload::to_zbytes;
use crate::publisher::{Publisher, PublisherOptions};
use crate::subscriber::Subscriber;
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Open a Zenoh session (default = peer mode).
///
/// `config` accepts a JSON5 string (a subset; plain JSON works too), a `Config`
/// instance, or nothing (default peer config). The string form is a convenience
/// over `Config.fromJson5`.
#[napi]
pub async fn open(config: Option<Either<String, &Config>>) -> Result<Session> {
  let config = match config {
    None => zenoh::Config::default(),
    Some(Either::A(json5)) => {
      zenoh::Config::from_json5(&json5).map_err(|e| zerr("open: parse config string", e))?
    }
    Some(Either::B(config)) => config.inner.clone(),
  };
  let session = zenoh::open(config)
    .await
    .map_err(|e| zerr("zenoh::open", e))?;
  Ok(Session { session })
}

/// QoS / metadata overrides for a session-level `put`. Unlike a declared
/// publisher (whose QoS is fixed), session `put` accepts per-call overrides.
/// (`timestamp` and `reliability` are deferred — see SPEC §9/§15.)
#[napi(object)]
pub struct PutOptions {
  pub encoding: Option<String>,
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub attachment: Option<Either<String, Uint8Array>>,
  pub allowed_destination: Option<Locality>,
}

/// QoS / metadata overrides for a session-level `delete` (no payload, so no
/// encoding). (`timestamp` and `reliability` are deferred — see SPEC §9/§15.)
#[napi(object)]
pub struct DeleteOptions {
  pub congestion_control: Option<CongestionControl>,
  pub priority: Option<Priority>,
  pub express: Option<bool>,
  pub attachment: Option<Either<String, Uint8Array>>,
  pub allowed_destination: Option<Locality>,
}

/// Snapshot of the session's view of the network, from `session.info()`.
#[napi(object)]
pub struct SessionInfo {
  /// Zenoh ID of this session.
  pub zid: String,
  /// Zenoh IDs of the routers this session is connected to.
  pub routers: Vec<String>,
  /// Zenoh IDs of the peers this session is connected to.
  pub peers: Vec<String>,
}

#[napi]
pub struct Session {
  session: zenoh::Session,
}

#[napi]
impl Session {
  /// Zenoh ID of this session.
  #[napi(getter)]
  pub fn zid(&self) -> String {
    self.session.zid().to_string()
  }

  /// This session's view of the network (its zid plus connected routers/peers).
  #[napi]
  pub async fn info(&self) -> SessionInfo {
    let info = self.session.info();
    let zid = info.zid().await.to_string();
    let routers = info.routers_zid().await.map(|z| z.to_string()).collect();
    let peers = info.peers_zid().await.map(|z| z.to_string()).collect();
    SessionInfo {
      zid,
      routers,
      peers,
    }
  }

  /// Session-level publication.
  #[napi]
  pub async fn put(
    &self,
    key_expr: String,
    payload: Either<String, Uint8Array>,
    options: Option<PutOptions>,
  ) -> Result<()> {
    let session = self.session.clone();
    let mut builder = session.put(key_expr, to_zbytes(payload));
    if let Some(o) = options {
      if let Some(encoding) = o.encoding {
        builder = builder.encoding(encoding.as_str());
      }
      if let Some(cc) = o.congestion_control {
        builder = builder.congestion_control(cc.into());
      }
      if let Some(priority) = o.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = o.express {
        builder = builder.express(express);
      }
      if let Some(attachment) = o.attachment {
        builder = builder.attachment(to_zbytes(attachment));
      }
      if let Some(destination) = o.allowed_destination {
        builder = builder.allowed_destination(destination.into());
      }
    }
    builder.await.map_err(|e| zerr("session.put", e))?;
    Ok(())
  }

  /// Session-level delete (tombstone).
  #[napi]
  pub async fn delete(&self, key_expr: String, options: Option<DeleteOptions>) -> Result<()> {
    let session = self.session.clone();
    let mut builder = session.delete(key_expr);
    if let Some(o) = options {
      if let Some(cc) = o.congestion_control {
        builder = builder.congestion_control(cc.into());
      }
      if let Some(priority) = o.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = o.express {
        builder = builder.express(express);
      }
      if let Some(attachment) = o.attachment {
        builder = builder.attachment(to_zbytes(attachment));
      }
      if let Some(destination) = o.allowed_destination {
        builder = builder.allowed_destination(destination.into());
      }
    }
    builder.await.map_err(|e| zerr("session.delete", e))?;
    Ok(())
  }

  /// Declare a publisher. QoS / addressing is fixed here; per-message overrides go
  /// on `publisher.put()`.
  #[napi]
  pub async fn declare_publisher(
    &self,
    key_expr: String,
    options: Option<PublisherOptions>,
  ) -> Result<Publisher> {
    let session = self.session.clone();
    let mut builder = session.declare_publisher(key_expr);
    if let Some(o) = options {
      if let Some(cc) = o.congestion_control {
        builder = builder.congestion_control(cc.into());
      }
      if let Some(priority) = o.priority {
        builder = builder.priority(priority.into());
      }
      if let Some(express) = o.express {
        builder = builder.express(express);
      }
      if let Some(encoding) = o.encoding {
        builder = builder.encoding(encoding.as_str());
      }
      if let Some(destination) = o.allowed_destination {
        builder = builder.allowed_destination(destination.into());
      }
    }
    let publisher = builder.await.map_err(|e| zerr("declare_publisher", e))?;
    let key_expr = publisher.key_expr().to_string();
    Ok(Publisher::new(key_expr, publisher))
  }

  /// Declare a channel-mode subscriber. The returned object is an async iterable:
  /// `for await (const sample of sub) { ... }`.
  #[napi]
  pub async fn declare_subscriber(&self, key_expr: String) -> Result<Subscriber> {
    let session = self.session.clone();
    let subscriber = session
      .declare_subscriber(key_expr)
      .await
      .map_err(|e| zerr("declare_subscriber", e))?;
    Ok(Subscriber::new(subscriber))
  }

  #[napi]
  pub async fn close(&self) -> Result<()> {
    let session = self.session.clone();
    session
      .close()
      .await
      .map_err(|e| zerr("session.close", e))?;
    Ok(())
  }
}
