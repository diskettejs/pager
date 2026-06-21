use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::error::to_napi_err;

/// Zenoh session configuration (mirrors `zenoh::Config`).
///
/// The configuration tree is opaque; construct it with one of the factory
/// methods. There is intentionally no public constructor.
#[napi]
pub struct Config {
  pub(crate) inner: zenoh::Config,
}

#[napi]
impl Config {
  /// The default configuration (peer mode, default endpoints).
  #[napi(factory)]
  // Inherent `default()` is the intended JS API (`Config.default()`), mirroring
  // `zenoh::Config::default()`; we don't want the `Default` trait here.
  #[allow(clippy::should_implement_trait)]
  pub fn default() -> Self {
    Self {
      inner: zenoh::Config::default(),
    }
  }

  /// Load configuration from a JSON5 string.
  #[napi(factory)]
  pub fn from_json5(json5: String) -> Result<Self> {
    Ok(Self {
      inner: zenoh::Config::from_json5(&json5).map_err(to_napi_err)?,
    })
  }

  /// Load configuration from a JSON5 file at `path`.
  #[napi(factory)]
  pub fn from_file(path: String) -> Result<Self> {
    Ok(Self {
      inner: zenoh::Config::from_file(&path).map_err(to_napi_err)?,
    })
  }

  /// Load configuration from the file pointed to by the `ZENOH_CONFIG`
  /// environment variable.
  #[napi(factory)]
  pub fn from_env() -> Result<Self> {
    Ok(Self {
      inner: zenoh::Config::from_env().map_err(to_napi_err)?,
    })
  }
}
