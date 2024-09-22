pub mod error;
pub mod keystore;

use std::sync::Arc;

pub use error::{Error, KeystoreError};
pub use keystore::{Backends, Keystore};

use crate::ShoelaceData;
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::get,
    Router,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use blake2::{Blake2s256, Digest};
use tracing::info;

/// Attaches the Proxy module to an Axum router
pub fn attach() -> Router<Arc<ShoelaceData>> {
    Router::new().route("/:id", get(serve))
}

/// Stores media URLs
#[tracing::instrument(err(Display), skip(url, data))]
pub async fn store(url: &str, data: ShoelaceData) -> Result<String, Error> {
    let hash = Blake2s256::digest(url.as_bytes());
    let hashstring = URL_SAFE.encode(hash).to_string();
    let hash_url = format!(
        "{}/proxy/{}",
        data.config.server.base_url,
        hashstring.clone()
    );

    let result = match &data.store {
        Keystore::Internal(store) => {
            let mut lock = store.lock().await;
            lock.insert(hashstring.clone(), url.to_string());
            Ok(hash_url)
        }
        Keystore::Redis(store) => {
            let mut con = store.to_owned();

            redis::cmd("SET")
                .arg(&[hashstring.clone(), url.to_string()])
                .query_async(&mut con)
                .await
                .map_err(KeystoreError::RedisError)?;
            Ok(hash_url)
        }
        Keystore::None => Ok(url.to_string()),
    };

    if !matches!(&data.store, Keystore::None) {
        info!(
            "Spawned hash {}{}",
            &hashstring,
            if data.config.logging.log_cdn {
                format!(" -> {}", url)
            } else {
                String::from("")
            }
        );
    }

    result
}

/// Proxies media from Threads
#[tracing::instrument(err(Display), fields(error, hash))]
async fn serve(
    Path(hash): Path<String>,
    State(data): State<Arc<ShoelaceData>>,
) -> Result<Response, Error> {
    let url: String = match &data.store {
        Keystore::Internal(store) => {
            let lock = store.lock().await;

            match lock.get(&hash) {
                Some(object) => object.to_owned(),
                None => return Err(Error::ObjectNotFound),
            }
        }
        Keystore::Redis(store) => {
            let mut con = store.to_owned();

            redis::cmd("GET")
                .arg(hash)
                .query_async(&mut con)
                .await
                .map_err(KeystoreError::RedisError)?
        }
        Keystore::None => return Err(Error::NoProxy),
    };

    let media = reqwest::get(url).await?.bytes().await?;
    let mime = infer::get(&media);

    if let Some(mime_type) = mime {
        Ok(Response::builder()
            .header("Content-Type", mime_type.to_string())
            .body(Body::from(media))?)
    } else {
        Err(Error::UnidentifiableMime)
    }
}
