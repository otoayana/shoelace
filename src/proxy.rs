use crate::{error::ProxyError, ShoelaceData};
use actix_web::{
    get,
    web::{Data, Path},
    HttpResponse,
};
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use blake2::{Blake2s256, Digest};
use reqwest::get;

/// Stores media URLs
pub async fn store(url: &str, data: Data<ShoelaceData>) -> String {
    if let Some(content) = &mut &data.internal_store {
        // Generates hash for URL in CDN
        let hash = Blake2s256::digest(url.as_bytes());
        let hashstring = URL_SAFE.encode(hash).to_string();
        let hash_url = format!("{}/proxy/{}", data.base_url, hashstring.clone());

        // Stores pair in keystore
        let mut lock = content.lock().await;
        lock.insert(hashstring.clone(), url.to_string());
	hash_url
    } else {
	url.to_string()
    }
}

/// Proxies media from Threads
#[get("/{image}")]
pub async fn proxy(
    path: Path<String>,
    data: Data<ShoelaceData>,
) -> Result<HttpResponse, ProxyError> {
    // Retrieves value from keystore
    if let Some(content) = &mut &data.internal_store {
        let lock = content.lock().await;
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
    } else {
        Err(ProxyError::NoProxy)
    }
}
