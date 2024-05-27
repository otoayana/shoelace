use std::env;

use crate::proxy::Backends;
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::fs::metadata;

//
// Information regarding every struct's functionality can be found in
// 'contrib/shoelace.toml', the sample config file for Shoelace.
//

// Settings structure
#[derive(Debug, Deserialize)]
pub(crate) struct Settings {
    pub(crate) server: Server,
    pub(crate) endpoint: Endpoint,
    pub(crate) proxy: Proxy,
    pub(crate) logging: Logging,
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
}

// Redis settings
#[derive(Debug, Deserialize)]
pub(crate) struct Redis {
    pub(crate) uri: String,
}

// Logging settings
#[derive(Debug, Deserialize)]
pub(crate) struct Logging {
    pub(crate) level: String,

    pub(crate) log_ips: bool,
    pub(crate) log_cdn: bool,

    pub(crate) store: bool,
    pub(crate) output: String,
}

// Implement constructor
impl Settings {
    pub(crate) fn new() -> Result<Self, ConfigError> {
        // Sets potential paths for config file
        let config_path = match env::var("SHOELACE_CONFIG") {
            Ok(path) => path,
            Err(_) => String::from("shoelace.toml"),
        };

        let maybe_file = metadata(&config_path);

        // Defines settings builder
        let mut builder = Config::builder()
            .add_source(Environment::with_prefix("SHOELACE"))
            // This will be the default setup, if no config files are provided.
            .set_default("server.listen", "0.0.0.0")?
            .set_default("server.port", "8080")?
            .set_default("server.base_url", "http://localhost:8080")?
            .set_default("server.tls.enabled", false)?
            .set_default("server.tls.cert", "")?
            .set_default("server.tls.key", "")?
            .set_default("endpoint.frontend", true)?
            .set_default("endpoint.api", true)?
            .set_default("proxy.backend", "internal")?
            .set_default("logging.level", "info")?
            .set_default("logging.log_ips", false)?
            .set_default("logging.log_cdn", false)?
            .set_default("logging.store", false)?
            .set_default("logging.output", "")?;

        if maybe_file.is_ok() {
            builder = builder.add_source(File::with_name(&config_path));
        }

        builder.build()?.try_deserialize()
    }
}
