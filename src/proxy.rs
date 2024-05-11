use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    web::{self, Data},
    HttpResponse,
};
use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use blake2::{Blake2s256, Digest};
use std::collections::HashMap;
use tokio::sync::Mutex;

// Database to save hash and URL pairs
pub struct Db {
    pub content: Mutex<HashMap<String, String>>,
}

/// Stores media URLs
#[tokio::main]
pub async fn store(url: &str, db: Data<Db>) -> Result<String> {
    // generates hash for url in cdn
    let hash = Blake2s256::digest(url.as_bytes());
    let hashstring = URL_SAFE.encode(hash);

    // stores pair in db
    let mut lock = db.content.lock().await;
    lock.insert(hashstring.to_string(), url.to_string());

    Ok(hashstring.to_string())
}

/// Proxies media from Threads
#[get("/{image}")]
pub async fn proxy(path: web::Path<String>, db: Data<Db>) -> actix_web::Result<HttpResponse> {
    // retrieves value from keystore
    let lock = db.content.lock().await;
    let url = match lock.get(&path.into_inner()) {
        Some(x) => x.to_owned(),
        None => String::new(),
    };

    // pipes through cdn
    let media = reqwest::get(url)
        .await
        .map_err(|_| actix_web::Error::from(ErrorNotFound("media not found")))?
        .bytes()
        .await
        .map_err(|_| actix_web::Error::from(ErrorInternalServerError("couldn't serve media")))?;

    let mime = infer::get(&media).expect("media unidentifiable").to_string();
    Ok(HttpResponse::Ok()
        .content_type(mime)
        .body(media))
}
