use std::path::Path;

use harp::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    #[serde(rename = "process_interval")]
    pub process_interval_secs: u64,
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub name: String,
    pub user: String,
    pub pass: String,
    pub host: String,
    pub port: i16,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let config_path = match path {
            Some(path) => path.as_ref().to_path_buf(),
            None => Path::new("/etc/harp/config.toml").to_path_buf(),
        };

        let config_file = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_file)?;

        Ok(config)
    }
}
