use std::env;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

// Settings structure
#[derive(Debug, Deserialize)]
pub(crate) struct Settings {
    pub(crate) server: Server,
    pub(crate) endpoint: Endpoint,
    pub(crate) proxy: Proxy,
}

// Server settings
#[derive(Debug, Deserialize)]
pub(crate) struct Server {
    pub(crate) listen: String,
    pub(crate) port: u16,
    pub(crate) base_url: String,
    pub(crate) tls: Tls,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Tls {
    pub(crate) enabled: bool,
    pub(crate) cert: String,
    pub(crate) key: String,
}

// Endpoint settings
#[derive(Debug, Deserialize)]
pub struct Endpoint {
    pub(crate) frontend: bool,
    pub(crate) api: bool,
}

// Proxy settings
#[derive(Debug, Deserialize)]
pub struct Proxy {
    pub(crate) backend: ProxyModes,
    pub(crate) redis: Option<Redis>,
    pub(crate) rocksdb: Option<RocksDB>,
}

// Redis settings
#[derive(Debug, Deserialize)]
pub struct Redis {
    pub(crate) uri: String,
}

// RocksDB settings
#[derive(Debug, Deserialize)]
pub struct RocksDB {
    pub(crate) path: String,
}

// Proxy modes
#[derive(Debug, Deserialize, Clone)]
pub enum ProxyModes {
    None,
    Internal,
    Redis,
    RocksDB,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let config_path = env::var("SHOELACE_CONFIG")
            .unwrap_or_else(|_| format!("{}/{}", env!("PWD"), "shoelace.toml"));

        let builder = Config::builder()
            .add_source(File::with_name(&config_path))
            .add_source(Environment::with_prefix("SHOELACE"))
            .build()?;

        builder.try_deserialize()
    }
}
