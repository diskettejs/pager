use crate::error::zerr;
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// Zenoh session configuration (default = peer mode). Wraps `zenoh::Config`.
///
/// This is the surface deployments use for connectivity — e.g. explicit
/// `connect`/`listen` endpoints when multicast discovery is unavailable between
/// processes. Constructed via the static factories; pass the result to `open()`.
#[napi]
pub struct Config {
  pub(crate) inner: zenoh::Config,
}

#[napi]
impl Config {
  /// Default configuration (peer mode).
  #[napi(factory)]
  pub fn default() -> Self {
    Config {
      inner: zenoh::Config::default(),
    }
  }

  /// Parse a JSON5 (or plain JSON, a subset of JSON5) configuration string.
  #[napi(factory)]
  pub fn from_json5(json5: String) -> Result<Self> {
    let inner = zenoh::Config::from_json5(&json5).map_err(|e| zerr("Config.fromJson5", e))?;
    Ok(Config { inner })
  }

  /// Load configuration from a file path (JSON5/JSON/YAML per extension).
  #[napi(factory)]
  pub fn from_file(path: String) -> Result<Self> {
    let inner = zenoh::Config::from_file(&path).map_err(|e| zerr("Config.fromFile", e))?;
    Ok(Config { inner })
  }

  /// Load configuration from the file referenced by the `ZENOH_CONFIG` env var.
  #[napi(factory)]
  pub fn from_env() -> Result<Self> {
    let inner = zenoh::Config::from_env().map_err(|e| zerr("Config.fromEnv", e))?;
    Ok(Config { inner })
  }

  /// Insert or override a value at a key path; `value` is a JSON5 fragment.
  /// e.g. `insertJson5("scouting/multicast/enabled", "false")`.
  #[napi]
  pub fn insert_json5(&mut self, key: String, value: String) -> Result<()> {
    self
      .inner
      .insert_json5(&key, &value)
      .map_err(|e| zerr("Config.insertJson5", e))?;
    Ok(())
  }

  /// Serialize the configuration to a JSON string (private fields elided).
  #[napi]
  #[allow(clippy::inherent_to_string)]
  pub fn to_string(&self) -> String {
    self.inner.to_string()
  }
}
