use napi_derive::napi;
use zenoh::config::Config as ZConfig;

#[napi]
pub struct Config {
  pub(crate) inner: ZConfig,
}

#[napi]
impl Config {
  /// The default configuration, used to open a session.
  #[napi(factory)]
  pub fn default() -> Self {
    Config {
      inner: ZConfig::default(),
    }
  }

  /// The default environment variable containing the file path used in `fromEnv`.
  #[napi]
  pub fn default_config_path_env() -> String {
    ZConfig::DEFAULT_CONFIG_PATH_ENV.to_string()
  }

  /// Load configuration from the file path specified in the
  /// `defaultConfigPathEnv` environment variable.
  #[napi(factory)]
  pub fn from_env() -> napi::Result<Self> {
    let inner = ZConfig::from_env().map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Config { inner })
  }

  /// Load configuration from the file at `path`.
  #[napi(factory)]
  pub fn from_file(path: String) -> napi::Result<Self> {
    let inner = ZConfig::from_file(path).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Config { inner })
  }

  /// Load configuration from the JSON5 string `input`.
  #[napi(factory)]
  pub fn from_json5(input: String) -> napi::Result<Self> {
    let inner = ZConfig::from_json5(&input).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Config { inner })
  }

  /// Returns a JSON string containing the configuration at `key`.
  #[napi]
  pub fn get_json(&self, key: String) -> napi::Result<String> {
    self
      .inner
      .get_json(&key)
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Inserts configuration value `value` at `key`.
  #[napi]
  pub fn insert_json5(&mut self, key: String, value: String) -> napi::Result<()> {
    self
      .inner
      .insert_json5(&key, &value)
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }

  /// Removes the configuration value at `key`.
  #[napi]
  pub fn remove(&mut self, key: String) -> napi::Result<()> {
    self
      .inner
      .remove(&key)
      .map_err(|e| napi::Error::from_reason(e.to_string()))
  }
}
