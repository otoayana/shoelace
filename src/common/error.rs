use std::{fmt::Display, time::SystemTimeError};

use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use config::ConfigError;
use spools::SpoolsError;
use thiserror::Error;
use tracing_log::log::SetLoggerError;

use crate::frontend::Base;

#[derive(Error, Debug)]
pub enum TimerError {
    ClockSkew,
    NotStarted,
}

impl Display for TimerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::ClockSkew => "clock skew",
            Self::NotStarted => "timer not started",
        };

        writeln!(f, "{}", message)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Threads(#[from] SpoolsError),
    #[error("proxy failed: {0}")]
    Proxy(#[from] crate::proxy::Error),
    #[error("template failed to render: {0}")]
    Template(#[from] askama::Error),
    #[error("couldn't fetch time: {0}")]
    Time(#[from] SystemTimeError),
    #[error("timer error: {0}")]
    TimerError(#[from] TimerError),
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
    pub fn into_plaintext(self) -> Response {
        let status = match self {
            Error::Threads(SpoolsError::NotFound(_)) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, self.to_string()).into_response()
    }

    fn status(&self) -> StatusCode {
        match self {
            Error::Threads(SpoolsError::NotFound(_)) | Error::NotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let base = Base::new().unwrap();
        let body: String;
        let status: StatusCode = self.status();

        let template = crate::frontend::templates::ErrorView {
            base,
            status: self.status().as_str(),
            error: self.to_string().as_str(),
        }
        .render();

        // Fallback in case the template fails to render.
        match template {
            Ok(template_body) => {
                body = template_body;
            }
            Err(error) => {
                body = format!("{}\n{}", error, self);
            }
        }

        (status, body).into_response()
    }
}
