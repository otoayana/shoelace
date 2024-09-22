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
#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub server: Server,
    pub endpoint: Endpoint,
    pub proxy: Proxy,
    pub logging: Logging,
}

// Server settings
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
    pub listen: String,
    pub port: u16,
    pub base_url: String,
    pub tls: Option<Tls>,
}

// TLS settings
#[derive(Clone, Debug, Deserialize)]
pub struct Tls {
    pub enabled: bool,
    pub cert: String,
    pub key: String,
}

// Endpoint settings
#[derive(Clone, Debug, Deserialize)]
pub struct Endpoint {
    pub frontend: bool,
    pub api: bool,
    pub rss: bool,
}

// Proxy settings
#[derive(Clone, Debug, Deserialize)]
pub struct Proxy {
    pub backend: Backends,
    pub redis: Option<Redis>,
}

// Redis settings
#[derive(Clone, Debug, Deserialize)]
pub struct Redis {
    pub uri: String,
}

// Logging settings
#[derive(Clone, Debug, Deserialize)]
pub struct Logging {
    pub level: String,

    pub log_ips: bool,
    pub log_cdn: bool,

    pub store: bool,
    pub output: String,
}

// Implement constructor
impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
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
            .set_default("endpoint.rss", true)?
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
