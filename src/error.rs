use std::time::SystemTimeError;

use actix_web::{
    error, get, http::{header::ContentType, StatusCode}, HttpResponse, ResponseError
};
use serde::{Deserialize, Serialize};
use spools::SpoolsError;
use tera::Context;
use thiserror::Error;

/// Defines frontend errors
#[derive(Error, Debug)]
pub enum ShoelaceError {
    #[error("{0}")]
    ThreadsError(#[from] SpoolsError),
    #[error("proxy failed: {0}")]
    ProxyError(#[from] ProxyError),
    #[error("template failed to render: {0}")]
    TemplateError(#[from] tera::Error),
    #[error("couldn't fetch time: {0}")]
    TimeError(#[from] SystemTimeError),
    #[error("not found")]
    NotFound,
}

/// Defines proxy errors
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("couldn't find object")]
    ObjectNotFound,
    #[error("endpoint error: {0}")]
    EndpointError(#[from] reqwest::Error),
    #[error("unable to identify mime type")]
    MimeError,
}

/// Constructs the contents for an error page
#[derive(Debug, Deserialize, Serialize)]
struct ErrorResponse {
    status_code: String,
    error: String,
}

impl error::ResponseError for ProxyError {
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
            Self::MimeError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl error::ResponseError for ShoelaceError {
    /// Fancy error method
    fn error_response(&self) -> HttpResponse {
        let body: String;
        let status_code: StatusCode;
        let template = crate::TEMPLATES
            .render(
                "common/error.html",
                &Context::from_serialize(ErrorResponse {
                    status_code: self.status_code().as_u16().to_string(),
                    error: self.to_string(),
                })
                .map_err(|err| ShoelaceError::TemplateError(err))
                .unwrap(),
            )
            .map_err(|err| ShoelaceError::TemplateError(err));

        if let Ok(template_body) = template {
            body = template_body;
            status_code = self.status_code()
        } else {
            body = format!("{}\n{}", template.unwrap_err(), self.to_string());
            status_code = StatusCode::INTERNAL_SERVER_ERROR;
        }

        HttpResponse::build(status_code)
            .insert_header(ContentType::html())
            .body(body)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            ShoelaceError::ThreadsError(SpoolsError::NotFound(_)) | ShoelaceError::NotFound => {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// Handles non-existant routes
pub async fn not_found() -> HttpResponse {
	ShoelaceError::NotFound.error_response()
}