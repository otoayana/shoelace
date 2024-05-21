use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use crate::{config::ProxyModes, error::ProxyError, ShoelaceData};
use blake2::{Blake2s256, Digest};
use reqwest::get;

/// Stores media URLs
pub async fn store(url: &str, data: Data<ShoelaceData>) -> Result<String, ProxyError> {
    // Find which keystore is being used
    // Generates hash for URL in CDN
    let hash = Blake2s256::digest(url.as_bytes());
    let hashstring = URL_SAFE.encode(hash).to_string();
    let hash_url = format!("{}/proxy/{}", data.base_url, hashstring.clone());
    match &data.keystore_type {
        ProxyModes::Internal => {
            if let Some(content) = &mut &data.internal_store {
                // Stores pair in keystore
                let mut lock = content.lock().await;
                lock.insert(hashstring.clone(), url.to_string());
                Ok(hash_url)
            } else {
                Ok(url.to_string())
            }
        }
        ProxyModes::RocksDB => {
            if let Some(rocks) = &data.rocksdb {
                rocks.put(hashstring, url).unwrap();
                Ok(hash_url)
            } else {
                Ok(url.to_string())
            }
        }
        ProxyModes::Redis => todo!(),
        ProxyModes::None => Ok(url.to_string()),
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

    match &data.keystore_type {
        ProxyModes::Internal => {
            if let Some(content) = &mut &data.internal_store {
                let lock = content.lock().await;
                url = match lock.get(&path.into_inner()) {
                    Some(object) => object.to_owned(),
                    None => return Err(ProxyError::ObjectNotFound),
                }
            } else {
                return Err(ProxyError::NoProxy);
            }
        }
        ProxyModes::RocksDB => {
            if let Some(rocks) = &data.rocksdb {
                match rocks.get(path.into_inner())? {
                    Some(value) => {
                        url = String::from_utf8(value)
                            .map_err(|_| ProxyError::CannotRetrieve)
                            .unwrap()
                    }
                    None => return Err(ProxyError::ObjectNotFound),
                }
            } else {
                return Err(ProxyError::NoProxy);
            }
        }
        ProxyModes::Redis => todo!(),
        ProxyModes::None => todo!(),
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
