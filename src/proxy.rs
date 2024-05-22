use crate::{error::ProxyError, KeyStore, ShoelaceData};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use blake2::{Blake2s256, Digest};
use reqwest::get;

/// Stores media URLs
pub async fn store(url: &str, data: Data<ShoelaceData>) -> Result<String, ProxyError> {
    // Generates hash for URL in CDN
    let hash = Blake2s256::digest(url.as_bytes());
    let hashstring = URL_SAFE.encode(hash).to_string();
    let hash_url = format!("{}/proxy/{}", data.base_url, hashstring.clone());

    // Find which keystore is being used
    match &data.store {
        Some(backend) => match backend {
            // Internal keystore
            KeyStore::Internal(store) => {
                let mut lock = store.lock().await;
                lock.insert(hashstring.clone(), url.to_string());
                Ok(hash_url)
            }
            // RocksDB
            KeyStore::RocksDB(store) => {
                store.put(hashstring, url).unwrap();
                Ok(hash_url)
            }
            // Redis
            KeyStore::Redis(store) => {
                let mut con = store.to_owned();

                redis::cmd("SET")
                    .arg(&[hashstring, url.to_string()])
                    .query_async(&mut con)
                    .await?;
                Ok(hash_url)
            }
        },
        None => Ok(url.to_string()),
    }
}

/// Proxies media from Threads
#[get("/{image}")]
pub async fn proxy(
    path: Path<String>,
    data: Data<ShoelaceData>,
) -> Result<HttpResponse, ProxyError> {
    // Retrieves value from keystore

    let url: String;

    match &data.store {
        Some(backend) => match backend {
            KeyStore::Internal(store) => {
                let lock = store.lock().await;
                url = match lock.get(&path.into_inner()) {
                    Some(object) => object.to_owned(),
                    None => return Err(ProxyError::ObjectNotFound),
                }
            }
            KeyStore::RocksDB(store) => match store.get(path.into_inner())? {
                Some(value) => {
                    url = String::from_utf8(value)
                        .map_err(|_| ProxyError::CannotRetrieve)
                        .unwrap()
                }
                None => return Err(ProxyError::ObjectNotFound),
            },
            KeyStore::Redis(store) => {
                let mut con = store.to_owned();

                url = redis::cmd("GET")
                    .arg(path.into_inner())
                    .query_async(&mut con)
                    .await?;
            }
        },
        None => return Err(ProxyError::NoProxy),
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
        Err(ProxyError::MimeError)
    }
}
