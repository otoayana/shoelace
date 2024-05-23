use crate::{config::Proxy, proxy::KeystoreError};
use core::fmt;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::Mutex;

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
    pub(crate) async fn new(config: Proxy) -> Result<Self, KeystoreError> {
        let backend = match config.backend {
            // RocksDB
            Backends::RocksDB => Self::RocksDB(
                // Checks if there's any settings set for RocksDB
                match config.rocksdb {
                    Some(rocksdb) => {
                        // Open keystore in the provided path
                        rocksdb::DB::open_default(rocksdb.path)?
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
                        let client = redis::Client::open(redis.uri).unwrap();

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
