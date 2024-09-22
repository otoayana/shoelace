use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

use crate::proxy::keystore::Backends;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Proxy is unavailable")]
    NoProxy,
    #[error("Couldn't find object")]
    ObjectNotFound,
    #[error("Endpoint error: {0}")]
    Endpoint(#[from] reqwest::Error),
    #[error("Unable to identify mime type")]
    UnidentifiableMime,
    #[error("Keystore error: {0}")]
    Keystore(#[from] KeystoreError),
    #[error("Web server error: {0}")]
    Web(#[from] axum::http::Error),
}

#[derive(Error, Debug)]
pub enum KeystoreError {
    #[error("{0}")]
    RedisError(#[from] redis::RedisError),
    #[error("invalid config for {0}")]
    InvalidConfig(Backends),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status: StatusCode = match self {
            Self::ObjectNotFound => StatusCode::NOT_FOUND,
            Self::Endpoint(ref val) => {
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
        };

        (status, self.to_string()).into_response()
    }
}
