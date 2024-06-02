use std::time::SystemTimeError;

use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
use git_version::git_version;
use serde::{Deserialize, Serialize};
use spools::SpoolsError;
use tera::Context;
use thiserror::Error;

/// Defines frontend errors
#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("{0}")]
    ThreadsError(#[from] SpoolsError),
    #[error("proxy failed: {0}")]
    ProxyError(#[from] crate::proxy::Error),
    #[error("template failed to render: {0}")]
    TemplateError(#[from] tera::Error),
    #[error("couldn't fetch time: {0}")]
    TimeError(#[from] SystemTimeError),
    #[error("not found")]
    NotFound,
}

/// Constructs the contents for an error page
#[derive(Debug, Deserialize, Serialize)]
struct ErrorResponse {
    base_url: String,
    status_code: String,
    error: String,
    rev: String,
}

// Plaintext error method
impl Error {
    pub(crate) fn to_plaintext(self) -> actix_web::Error {
        match self {
            Error::ThreadsError(spools_err) => {
                if let SpoolsError::NotFound(_) = spools_err {
                    error::ErrorNotFound(spools_err)
                } else {
                    error::ErrorInternalServerError(spools_err)
                }
            }
            _ => error::ErrorInternalServerError(self),
        }
    }
}

// Fancy error trait
impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        // Defines return values
        let body: String;
        let status_code: StatusCode;

        // Renders error template
        let template = crate::TEMPLATES
            .render(
                "common/error.html",
                &Context::from_serialize(ErrorResponse {
                    base_url: String::new(),
                    status_code: self.status_code().as_u16().to_string(),
                    error: self.to_string(),
                    rev: git_version!(
                        args = ["--always", "--dirty=-dirty"],
                        fallback = format!("v{}", env!("CARGO_PKG_VERSION"))
                    )
                    .to_string(), // Needs to be redefined, since in this scope we can't read application data
                })
                .map_err(Error::TemplateError)
                .unwrap(),
            )
            .map_err(Error::TemplateError);

        // Fallback in case the template fails to render.
        if let Ok(template_body) = template {
            body = template_body;
            status_code = self.status_code()
        } else {
            body = format!("{}\n{}", template.unwrap_err(), self);
            status_code = StatusCode::INTERNAL_SERVER_ERROR;
        }

        // Send response
        HttpResponse::build(status_code)
            .insert_header(ContentType::html())
            .body(body)
    }

    // Map error codes
    fn status_code(&self) -> StatusCode {
        match self {
            Error::ThreadsError(SpoolsError::NotFound(_)) | Error::NotFound => {
                StatusCode::NOT_FOUND
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// Handles non-existant routes
pub(crate) async fn not_found(front: bool) -> HttpResponse {
    // Will either serve a fancy or plaintext version, depending on whether the frontend is enabled
    if front {
        Error::NotFound.error_response()
    } else {
        HttpResponse::NotFound().body("not found")
    }
}
