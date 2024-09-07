use std::time::SystemTimeError;

use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
use config::ConfigError;
use git_version::git_version;
use serde::{Deserialize, Serialize};
use spools::SpoolsError;
use tera::Context;
use thiserror::Error;
use tracing_log::log::SetLoggerError;

// Defines frontend errors
#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("{0}")]
    Threads(#[from] SpoolsError),
    #[error("proxy failed: {0}")]
    Proxy(#[from] crate::proxy::Error),
    #[error("(legacy) template failed to render: {0}")]
    LegacyTemplate(#[from] tera::Error),
    #[error("template failed to render: {0}")]
    Template(#[from] askama::Error),
    #[error("couldn't fetch time: {0}")]
    Time(#[from] SystemTimeError),
    #[error("couldn't start logger: {0}")]
    Logger(#[from] SetLoggerError),
    #[error("config error: {0}")]
    Config(#[from] ConfigError),
    #[error("not found")]
    NotFound,
}

// Constructs the contents for an error page
#[derive(Debug, Deserialize, Serialize)]
struct ErrorResponse {
    base_url: String,
    status_code: String,
    error: String,
    rev: String,
}

// Plaintext error method
impl Error {
    pub(crate) fn into_plaintext(self) -> actix_web::Error {
        match self {
            Error::Threads(spools_err) => {
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
        /*let template = crate::TEMPLATES
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
                .map_err(Error::LegacyTemplate)
                .unwrap(),
            )
            .map_err(Error::LegacyTemplate);*/
        let template: Result<String, Error> = Ok(self.to_string());

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
            Error::Threads(SpoolsError::NotFound(_)) | Error::NotFound => StatusCode::NOT_FOUND,
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
