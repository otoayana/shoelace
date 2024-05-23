use std::env;

use crate::proxy::Backends;
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
    pub(crate) tls: Option<Tls>,
}

// TLS settings
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
    pub(crate) backend: Backends,
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

// Implement constructor
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        // Sets potential paths for config file
        let config_path = match env::var("SHOELACE_CONFIG") {
            Ok(path) => path,
            Err(_) => String::from("shoelace.toml"),
        };

        // Defines settings builder
        let builder = Config::builder()
            .add_source(Environment::with_prefix("SHOELACE"))
            .add_source(File::with_name(&config_path))
            // This will be the default setup, if no config files are provided.
            .set_default("server.listen", "0.0.0.0")?
            .set_default("server.port", "8080")?
            .set_default("server.tls.enabled", false)?
            .set_default("endpoint.frontend", true)?
            .set_default("endpoint.api", true)?
            .set_default("proxy.backend", "internal")?
            .build()?;

        builder.try_deserialize()
    }
}
