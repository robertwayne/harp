use std::{
    net::{IpAddr, SocketAddr},
    num::{NonZeroU32, NonZeroU64},
    path::Path,
};

use harp::Result;
use serde::Deserialize;

/// A struct representing the configuration for the harpd daemon.
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    host: IpAddr,
    port: u16,
    database: DatabaseConfig,

    // Duration in seconds between processing the queue.
    #[serde(rename = "process_interval")]
    pub process_interval_secs: NonZeroU64,

    // Maximum size (in bytes) to accept for a single packet.
    #[serde(default = "default_max_packet_size")]
    pub max_packet_size: usize,
}

#[derive(Debug, Deserialize)]
struct DatabaseConfig {
    name: String,
    user: String,
    pass: String,
    host: IpAddr,
    port: i16,

    // Maximum number of connections to assign to the database connection pool.
    max_connections: NonZeroU32,
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

    /// Returns a full connection string for the database.
    pub(crate) fn get_database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database.user,
            self.database.pass,
            self.database.host,
            self.database.port,
            self.database.name
        )
    }

    /// Returns a `SocketAddr` for the Harp server.
    pub(crate) fn get_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    /// Returns the maximum connections to be assigned to
    /// the database connection pool.
    pub(crate) fn get_max_connections(&self) -> u32 {
        self.database.max_connections.into()
    }

    /// Returns the interval in seconds between processing the queue.
    pub(crate) fn get_process_interval_secs(&self) -> u64 {
        self.process_interval_secs.into()
    }
}

fn default_max_packet_size() -> usize {
    1024
}
