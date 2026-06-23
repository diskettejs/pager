use napi_derive::napi;
use zenoh::config::WhatAmI as ZWhatAmI;

#[napi(string_enum)]
pub enum WhatAmI {
  Router,
  Peer,
  Client,
}

impl From<WhatAmI> for ZWhatAmI {
  fn from(value: WhatAmI) -> Self {
    match value {
      WhatAmI::Router => Self::Router,
      WhatAmI::Peer => Self::Peer,
      WhatAmI::Client => Self::Client,
    }
  }
}

impl From<ZWhatAmI> for WhatAmI {
  fn from(value: ZWhatAmI) -> Self {
    match value {
      ZWhatAmI::Router => Self::Router,
      ZWhatAmI::Peer => Self::Peer,
      ZWhatAmI::Client => Self::Client,
    }
  }
}
