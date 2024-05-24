pub(crate) mod error;
pub(crate) mod keystore;

pub(crate) use error::{Error, KeystoreError};
pub(crate) use keystore::{Backends, Keystore};

use crate::ShoelaceData;
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use blake2::{Blake2s256, Digest};
use reqwest::get;
use tracing::info;

// Stores media URLs
#[tracing::instrument(err(Display), skip(url, data))]
pub async fn store(url: &str, data: Data<ShoelaceData>) -> Result<String, Error> {
    // Generates hash for URL in CDN
    let hash = Blake2s256::digest(url.as_bytes());
    let hashstring = URL_SAFE.encode(hash).to_string();
    let hash_url = format!("{}/proxy/{}", data.base_url, hashstring.clone());

    // Find which keystore is being used
    let result = match &data.store {
        // Internal keystore
        Keystore::Internal(store) => {
            let mut lock = store.lock().await;
            lock.insert(hashstring.clone(), url.to_string());
            Ok(hash_url)
        }
        // RocksDB
        Keystore::RocksDB(store) => {
            store.put(hashstring.clone(), url).unwrap();
            Ok(hash_url)
        }
        // Redis
        Keystore::Redis(store) => {
            let mut con = store.to_owned();

            redis::cmd("SET")
                .arg(&[hashstring.clone(), url.to_string()])
                .query_async(&mut con)
                .await
                .map_err(|err| KeystoreError::RedisError(err))?;
            Ok(hash_url)
        }
        Keystore::None => Ok(url.to_string()),
    };

    if !matches!(&data.store, Keystore::None) {
        info!("Spawned hash {}", &hashstring)
    }

    result
}

// Proxies media from Threads
#[tracing::instrument(err(Display), fields(error, path))]
#[get("/{image}")]
pub async fn serve(path: Path<String>, data: Data<ShoelaceData>) -> Result<HttpResponse, Error> {
    let url: String;

    // Retrieves value from keystore
    match &data.store {
        Keystore::Internal(store) => {
            // Lock hash map
            let lock = store.lock().await;

            url = match lock.get(&path.into_inner()) {
                Some(object) => object.to_owned(),
                None => return Err(Error::ObjectNotFound),
            }
        }
        Keystore::RocksDB(store) => match store
            .get(path.into_inner())
            .map_err(|err| KeystoreError::RocksDBError(err))?
        {
            Some(value) => {
                url = String::from_utf8(value)
                    .map_err(|_| Error::CannotRetrieve)
                    .unwrap()
            }
            None => return Err(Error::ObjectNotFound),
        },
        Keystore::Redis(store) => {
            let mut con = store.to_owned();

            url = redis::cmd("GET")
                .arg(path.into_inner())
                .query_async(&mut con)
                .await
                .map_err(|err| KeystoreError::RedisError(err))?;
        }
        Keystore::None => return Err(Error::NoProxy),
    }

    // Pipes request to CDN
    let media = get(url).await?.bytes().await?;

    // Identifies MIME type
    let mime = infer::get(&media);

    if let Some(mime_type) = mime {
        Ok(HttpResponse::Ok()
            .content_type(mime_type.to_string())
            .body(media))
    } else {
        Err(Error::MimeError)
    }
}
