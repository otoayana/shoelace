use crate::{config::Proxy, proxy::KeystoreError};
use core::fmt;
use redis::ConnectionAddr;
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub enum Keystore {
    Internal(Arc<Mutex<HashMap<String, String>>>),
    Redis(redis::aio::MultiplexedConnection),
    None,
}

#[derive(Debug, Deserialize, Clone)]
pub enum Backends {
    None,
    Internal,
    Redis,
}

impl Keystore {
    /// Builds a new Keystore object
    #[tracing::instrument(name = "init", skip(config))]
    pub async fn new(config: Proxy) -> Result<Self, KeystoreError> {
        let mut redis_conninfo: Option<ConnectionAddr> = None;

        let backend = match config.backend {
            Backends::Redis => Self::Redis({
                match config.redis {
                    Some(redis) => {
                        let client = redis::Client::open(redis.uri)
                            .map_err(KeystoreError::RedisError)
                            .unwrap();

                        redis_conninfo = Some(client.clone().get_connection_info().clone().addr);

                        client.get_multiplexed_async_connection().await?
                    }
                    None => return Err(KeystoreError::InvalidConfig(config.backend)),
                }
            }),
            // The internal keystore is essentially just a HashMap, which is kept in memory
            Backends::Internal => Self::Internal(Arc::new(Mutex::new(HashMap::new()))),
            Backends::None => Self::None,
        };

        if !matches!(backend, Self::None) {
            info!(
                "Connected to {} keystore {}",
                &config.backend,
                if let Backends::Redis = &config.backend {
                    format!(
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
                    )
                } else {
                    String::new()
                }
            );
        } else {
            warn!("No keystore backend. Proxy has been disabled")
        }
        Ok(backend)
    }
}

impl fmt::Display for Backends {
    fn fmt(&self, f: &mut fmt::Formatter) -> std::fmt::Result {
        let out = match self {
            Backends::Redis => "Redis",
            Backends::Internal => "Internal",
            Backends::None => "None",
        };

        write!(f, "{}", out)
    }
}
