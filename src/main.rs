mod api;
mod common;
mod frontend;
mod proxy;
mod rss;

use axum_server::tls_rustls::RustlsConfig;
pub(crate) use common::config;
pub(crate) use common::error::Error;
pub(crate) use common::req;
use tracing_log::LogTracer;

#[cfg(test)]
mod test;

use crate::common::config::{Settings, Tls};
use anyhow::Result;
use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    RequestPartsExt, Router,
};
use git_version::git_version;
use lazy_static::lazy_static;
use proxy::Keystore;
use std::{
    fs::File,
    net::SocketAddr,
    process::id,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{info, instrument, warn};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter, Registry};

#[derive(Debug)]
pub(crate) struct ShoelaceData {
    pub(crate) store: Keystore,
    pub(crate) log_cdn: bool,
    log_ips: bool,
    frontend: bool,
    pub(crate) base_url: String,
}

lazy_static! {
    pub static ref REVISION: &'static str = git_version!(
        args = ["--always", "--dirty=-dirty"],
        fallback = format!("v{}", env!("CARGO_PKG_VERSION"))
    )
    .trim_end_matches(".0");
}

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[instrument(name = "web", skip(state, request, next))]
async fn logger<'a>(
    State(state): State<Arc<ShoelaceData>>,
    request: axum::extract::Request,
    next: Next,
) -> axum::response::Response {
    let start = SystemTime::now();
    let tse_start: Option<Duration> = match start.duration_since(UNIX_EPOCH) {
        Ok(time) => Some(time),
        Err(_) => None,
    };

    let (parts, body) = request.into_parts();
    let mut inner_parts = parts.clone();

    let ip: String = if state.log_ips {
        format!(
            " {} ",
            match inner_parts.extract::<ConnectInfo<SocketAddr>>().await {
                Ok(ip) => ip.ip().to_string(),
                Err(_) => "unknown".to_string(),
            },
        )
    } else {
        " ".to_string()
    };

    let rebuilt_req = Request::from_parts(parts, body);
    let uri = rebuilt_req.uri().clone();

    let response = next.run(rebuilt_req).await;
    let (res_parts, res_body) = response.into_parts();
    let res = Response::from_parts(res_parts, res_body);

    let status = res.status();
    let status_chunk = if status == StatusCode::OK {
        "<3"
    } else if status == StatusCode::NOT_FOUND {
        "??"
    } else {
        "!!"
    };

    let end = SystemTime::now();
    let tse_end: Option<Duration> = match end.duration_since(UNIX_EPOCH) {
        Ok(time) => Some(time),
        Err(_) => None,
    };

    let duration = if let (Some(e), Some(s)) = (tse_end, tse_start) {
        format!("{:?}", e - s)
    } else {
        String::new()
    };

    let message = format!(
        "{} {} {}{}{}",
        status_chunk,
        status.as_u16(),
        uri,
        ip,
        duration
    );

    if status.as_u16() < 500 {
        info!("{}", message)
    } else {
        warn!("{}", message)
    };

    res
}

async fn not_found(State(state): State<Arc<ShoelaceData>>) -> (StatusCode, Body) {
    (
        StatusCode::NOT_FOUND,
        if state.frontend {
            Error::NotFound.into_response().into_body()
        } else {
            Error::NotFound.into_plaintext().into_body()
        },
    )
}

// Web server
#[instrument(name = "shoelace::main")]
#[tokio::main]
async fn main() -> Result<()> {
    let config = Settings::new()?;

    let filter = EnvFilter::builder()
        .with_default_directive(
            match config.logging.level.as_str() {
                "error" => LevelFilter::ERROR,
                "warn" => LevelFilter::WARN,
                "debug" => LevelFilter::DEBUG,
                "trace" => LevelFilter::TRACE,
                "info" | _ => LevelFilter::INFO,
            }
            .into(),
        )
        .from_env()?;

    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    let registry = Registry::default()
        .with(if config.logging.store {
            let file = File::create(config.logging.output)?;
            Some(Layer::default().with_writer(Mutex::new(file)))
        } else {
            None
        })
        .with(Layer::default().with_writer(non_blocking))
        .with(filter);

    tracing::subscriber::set_global_default(registry)?;
    LogTracer::init()?;

    info!(
        "👟 Shoelace {} | PID: {} | https://sr.ht/~nixgoat/shoelace",
        REVISION.to_string(),
        id()
    );

    let data = Arc::new(ShoelaceData {
        store: Keystore::new(config.proxy).await?,
        log_cdn: config.logging.log_cdn,
        log_ips: config.logging.log_ips,
        frontend: config.endpoint.frontend,
        base_url: config.server.base_url.clone(),
    });

    info!("Base URL is set to {}", config.server.base_url);

    if !config.endpoint.frontend {
        warn!("Frontend has been disabled");
    }

    if !config.endpoint.api {
        warn!("API has been disabled");
    }

    let app = Router::new()
        .nest("/api/", api::attach(config.endpoint.api))
        .nest("/rss/", rss::attach(config.endpoint.rss))
        .nest("/proxy/", proxy::attach())
        .merge(frontend::routes::attach(config.endpoint.frontend))
        .layer(middleware::from_fn_with_state(data.clone(), logger))
        .fallback(not_found)
        .with_state(data);

    let tls_params = match config.server.tls {
        Some(opt) => {
            if opt.enabled {
                info!("TLS has been enabled");
            }

            opt
        }
        None => Tls {
            enabled: false,
            cert: String::new(),
            key: String::new(),
        },
    };

    info!(
        "Accepting connections at {}:{}",
        config.server.listen, config.server.port
    );

    if !tls_params.enabled {
        axum_server::bind(format!("{}:{}", config.server.listen, config.server.port).parse()?)
            .serve(app.into_make_service_with_connect_info::<SocketAddr>())
            .await?
    } else {
        let tls_config = RustlsConfig::from_pem_file(tls_params.cert, tls_params.key).await?;

        axum_server::bind_rustls(
            format!("{}:{}", config.server.listen, config.server.port).parse()?,
            tls_config,
        )
        .serve(app.into_make_service())
        .await?
    };

    info!("🚪 Shoelace exited successfully. See you soon!");
    Ok(())
}
