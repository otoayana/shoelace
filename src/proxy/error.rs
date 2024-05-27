use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use thiserror::Error;

use crate::proxy::keystore::Backends;

/// Defines proxy errors
#[derive(Error, Debug)]
pub enum Error {
    #[error("Proxy is unavailable")]
    NoProxy,
    #[error("Couldn't find object")]
    ObjectNotFound,
    #[error("Endpoint error: {0}")]
    EndpointError(#[from] reqwest::Error),
    #[error("Unable to identify mime type")]
    MimeError,
    #[error("Keystore error: {0}")]
    KeystoreError(#[from] KeystoreError),
}

// Defines keystore errors
#[derive(Error, Debug)]
pub enum KeystoreError {
    #[error("{0}")]
    RedisError(#[from] redis::RedisError),
    #[error("invalid config for {0}")]
    InvalidConfig(Backends),
}

// Defines plaintext error responses for proxy
impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code()).body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::ObjectNotFound => StatusCode::NOT_FOUND,
            Self::EndpointError(val) => {
                if let Some(status) = val.status() {
                    match status {
                        reqwest::StatusCode::NOT_FOUND => StatusCode::NOT_FOUND,
                        _ => StatusCode::BAD_GATEWAY,
                    }
                } else {
                    StatusCode::BAD_GATEWAY
                }
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
