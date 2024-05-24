use crate::{config::Proxy, proxy::KeystoreError};
use core::fmt;
use redis::ConnectionAddr;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{info, warn};

// Define keystores
#[derive(Debug)]
pub(crate) enum Keystore {
    Internal(Mutex<HashMap<String, String>>),
    RocksDB(rocksdb::DB),
    Redis(redis::aio::MultiplexedConnection),
    None,
}

// Possible backends
#[derive(Debug, Deserialize, Clone)]
pub(crate) enum Backends {
    None,
    Internal,
    Redis,
    RocksDB,
}

impl Keystore {
    #[tracing::instrument(name = "init", skip(config))]
    pub(crate) async fn new(config: Proxy) -> Result<Self, KeystoreError> {
        let mut redis_conninfo: Option<ConnectionAddr> = None;

        let backend = match config.backend {
            // RocksDB
            Backends::RocksDB => Self::RocksDB(
                // Checks if there's any settings set for RocksDB
                match &config.rocksdb {
                    Some(rocksdb) => {
                        // Open keystore in the provided path
                        rocksdb::DB::open_default(rocksdb.path.clone())?
                    }
                    None => return Err(KeystoreError::InvalidConfig(config.backend)),
                },
            ),
            // Redis
            Backends::Redis => Self::Redis({
                // Checks if there's any settings set for Redis
                match config.redis {
                    Some(redis) => {
                        // Configure client using the URI provided by the user
                        let client = redis::Client::open(redis.uri)
                            .map_err(|err| KeystoreError::RedisError(err))
                            .unwrap();

                        redis_conninfo = Some(client.clone().get_connection_info().clone().addr);

                        // Establish connection
                        client.get_multiplexed_async_connection().await?
                    }
                    None => return Err(KeystoreError::InvalidConfig(config.backend)),
                }
            }),
            // Internal (Creates hash map)
            Backends::Internal => Self::Internal(Mutex::new(HashMap::new())),
            // No backend
            Backends::None => Self::None,
        };

        if !matches!(backend, Self::None) {
            info!(
                "Connected to {} keystore {}",
                &config.backend,
                match &config.backend {
                    Backends::RocksDB => format!("at {}", &config.rocksdb.unwrap().path),
                    Backends::Redis => format!(
                        "at {}",
                        match redis_conninfo.unwrap() {
                            ConnectionAddr::Tcp(host, port) => format!("redis://{}:{}", host, port),
                            ConnectionAddr::TcpTls {
                                host,
                                port,
                                insecure: _,
                                tls_params: _,
                            } => format!("redis://{}:{} (TLS)", host, port),
                            ConnectionAddr::Unix(path) =>
                                format!("redis+unix://{}", path.display()),
                        }
                    ),
                    _ => String::new(),
                }
            );
        } else {
            warn!("No keystore backend. Proxy has been disabled")
        }
        Ok(backend)
    }
}

// Implement graceful shutdown
impl Drop for Keystore {
    fn drop(&mut self) {
        // Only RocksDB needs it, in order to close unfinished connections before shutting down
        if let Self::RocksDB(val) = self {
            val.cancel_all_background_work(true)
        }
    }
}

impl fmt::Display for Backends {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        let out = match self {
            Backends::RocksDB => "RocksDB",
            Backends::Redis => "Redis",
            Backends::Internal => "Internal",
            Backends::None => "None",
        };

        write!(f, "{}", out)
    }
}
