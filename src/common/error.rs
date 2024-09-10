use std::time::SystemTimeError;

use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse, ResponseError,
};
use askama::Template;
use config::ConfigError;
use spools::SpoolsError;
use thiserror::Error;
use tracing_log::log::SetLoggerError;

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
    #[error("pattern error: {0}")]
    Pattern(#[from] regex::Error),
    #[error("not found")]
    NotFound,
}

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

impl error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        let base = crate::frontend::Base::new().unwrap();
        let body: String;
        let status: StatusCode;
        let template = crate::frontend::ErrorView {
            base,
            status: self.status_code().as_str(),
            error: self.to_string().as_str(),
        }
        .render();

        // Fallback in case the template fails to render.
        match template {
            Ok(template_body) => {
                body = template_body;
                status = self.status_code()
            }
            Err(error) => {
                body = format!("{}\n{}", error, self);
                status = StatusCode::INTERNAL_SERVER_ERROR;
            }
        }

        HttpResponse::build(status)
            .insert_header(ContentType::html())
            .body(body)
    }

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
