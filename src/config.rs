use std::env;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub (crate) struct Settings {
	pub (crate) server: Server,
	pub (crate) endpoint: Endpoint,
	pub (crate) proxy: Proxy,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub (crate) struct Server {
	pub (crate) listen: String,
	pub (crate) port: u16,
	pub (crate) base_url: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Endpoint {
	pub (crate) frontend: bool,
	pub (crate) api: bool,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Proxy {
	pub (crate) mode: ProxyModes,
	pub (crate) redis: Option<Redis>,
	pub (crate) rocksdb: Option<RocksDB>,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Redis {
	pub (crate) address: String,
	pub (crate) port: u16,
	pub (crate) username: Option<String>,
	pub (crate) password: Option<String>
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct RocksDB {
	pub (crate) path: String
}

#[derive(Debug, Deserialize, Clone)]
pub enum ProxyModes {
	None,
	Internal,
	Redis,
	RocksDB
}

impl Settings {
	pub fn new() -> Result<Self, ConfigError>  {
		let config_path = env::var("SHOELACE_CONFIG").unwrap_or_else(|_| format!("{}/{}", env!("CARGO_MANIFEST_DIR"), "shoelace.toml"));
		
		let builder = Config::builder().add_source(File::with_name(&config_path)).add_source(Environment::with_prefix("SHOELACE")).build()?;
		
		builder.try_deserialize()
	}
}