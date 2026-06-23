use napi::Either;
use napi_derive::napi;
use zenoh::query::Reply as ZReply;
use zenoh_ext::RepliesConfig as ZRepliesConfig;

use crate::entity_global_id::EntityGlobalId;
use crate::qos::{CongestionControl, Priority};
use crate::reply_error::ReplyError;
use crate::sample::Sample;

#[napi]
pub struct Reply {
  pub(crate) inner: ZReply,
}

impl Reply {
  /// Internal constructor contract: wrap an owned `zenoh` value.
  pub(crate) fn from_inner(inner: ZReply) -> Self {
    Reply { inner }
  }
}

#[napi]
impl Reply {
  #[napi]
  pub fn result(&self) -> Either<Sample, ReplyError> {
    match self.inner.result() {
      Ok(s) => Either::A(Sample::new(s.clone())),
      Err(e) => Either::B(ReplyError::from_inner(e.clone())),
    }
  }

  #[napi(getter)]
  pub fn replier_id(&self) -> Option<EntityGlobalId> {
    self.inner.replier_id().map(EntityGlobalId::from_inner)
  }
}

#[napi(object)]
pub struct RepliesConfig {
  /// Priority of reply samples (default: `Data`).
  pub priority: Option<Priority>,
  /// Congestion control for reply samples (default: `Block`).
  pub congestion_control: Option<CongestionControl>,
  /// When `true`, reply samples are sent unbatched.
  pub express: Option<bool>,
}

impl RepliesConfig {
  pub(crate) fn into_zenoh(self) -> zenoh_ext::RepliesConfig {
    let mut config = zenoh_ext::RepliesConfig::default();
    if let Some(priority) = self.priority {
      config = config.priority(priority.into());
    }
    if let Some(congestion_control) = self.congestion_control {
      config = config.congestion_control(congestion_control.into());
    }
    if let Some(express) = self.express {
      config = config.express(express);
    }
    config
  }
}
