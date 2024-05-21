use crate::error::ProxyError;
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use blake2::{Blake2s256, Digest};
use reqwest::get;
use std::collections::HashMap;
use tokio::sync::Mutex;

// Database to save hash and URL pairs
pub struct KeyStore {
    pub content: Mutex<HashMap<String, String>>,
}

/// Stores media URLs
pub async fn store(url: &str, db: Data<KeyStore>) -> String {
    // Generates hash for URL in CDN
    let hash = Blake2s256::digest(url.as_bytes());
    let hashstring = URL_SAFE.encode(hash);

    // Stores pair in keystore
    let mut lock = db.content.lock().await;
    lock.insert(hashstring.to_string(), url.to_string());

    hashstring.to_string()
}

/// Proxies media from Threads
#[get("/{image}")]
pub async fn proxy(path: Path<String>, db: Data<KeyStore>) -> Result<HttpResponse, ProxyError> {
    // Retrieves value from keystore
    let lock = db.content.lock().await;
    let url = match lock.get(&path.into_inner()) {
        Some(object) => object.to_owned(),
        None => return Err(ProxyError::ObjectNotFound),
    };

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
