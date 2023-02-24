use std::path::Path;

use harp::Result;
use serde::Deserialize;

/// A struct representing the configuration for the harpd daemon.
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub host: String,
    pub port: u16,
    // Duration in seconds between processing the queue.
    #[serde(rename = "process_interval")]
    pub process_interval_secs: u64,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DatabaseConfig {
    pub name: String,
    pub user: String,
    pub pass: String,
    pub host: String,
    pub port: i16,
}

impl Config {
    /// Attempts to read a given config file. If no file is given, it will
    /// attempt to read the default config file at `/etc/harp/config.toml`.
    ///
    /// # Example
    ///
    /// ```toml
    /// # config.toml
    /// host = "127.0.0.1"
    /// port = 7777
    /// process_interval = 10
    ///
    /// [database]
    /// name = "harp"
    /// user = "harp"
    /// pass = "harp"
    /// host = "127.0.0.1"
    /// port = 5432
    /// ```
    ///
    /// See [Config] for more information.
    pub(crate) fn load_from_file<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let config_path = match path {
            Some(path) => path.as_ref().to_path_buf(),
            None => Path::new("/etc/harp/config.toml").to_path_buf(),
        };

        let config_file = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_file)?;

        Ok(config)
    }
}
