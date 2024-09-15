mod api;
mod common;
mod frontend;
mod proxy;
mod rss;

use actix_web::http::StatusCode;
use axum_server::tls_rustls::RustlsConfig;
pub(crate) use common::config;
pub(crate) use common::error::Error;
pub(crate) use common::req;
use tracing_log::LogTracer;

#[cfg(test)]
mod test;

use crate::common::config::{Settings, Tls};
use axum::Router;
use git_version::git_version;
use lazy_static::lazy_static;
use proxy::Keystore;
use std::{
    fs::File,
    io::{self, ErrorKind},
    process::id,
    sync::{Arc, Mutex},
};
use tracing::{info, instrument, warn};
use tracing_subscriber::{fmt::Layer, prelude::*, EnvFilter, Registry};

#[derive(Debug)]
pub(crate) struct ShoelaceData {
    pub(crate) store: Keystore,
    pub(crate) log_cdn: bool,
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

fn _log_err<'a>(status: StatusCode) -> &'a str {
    if status == StatusCode::NOT_FOUND {
        "??"
    } else if status == StatusCode::OK {
        "<3"
    } else {
        "!!"
    }
}

// Web server
#[instrument(name = "shoelace::main")]
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = Settings::new().map_err(|err| io::Error::new(ErrorKind::InvalidInput, err))?;

    let filter = EnvFilter::builder()
        .from_env()
        .map_err(|err| io::Error::new(ErrorKind::Other, err))?
        .add_directive(
            "none"
                .parse()
                .map_err(|err| io::Error::new(ErrorKind::Other, err))?,
        )
        .add_directive(
            format!("shoelace={}", config.logging.level)
                .parse()
                .map_err(|err| io::Error::new(ErrorKind::Other, err))?,
        );

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

    tracing::subscriber::set_global_default(registry).unwrap();
    LogTracer::init().map_err(|err| io::Error::new(ErrorKind::Other, err))?;

    info!(
        "👟 Shoelace {} | PID: {} | https://sr.ht/~nixgoat/shoelace",
        REVISION.to_string(),
        id()
    );

    let data = Arc::new(ShoelaceData {
        store: Keystore::new(config.proxy)
            .await
            .map_err(|err| io::Error::new(ErrorKind::ConnectionRefused, err))?,
        log_cdn: config.logging.log_cdn,
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
        .merge(frontend::routes::attach(config.endpoint.frontend))
        .with_state(data);

    // let mut server = HttpServer::new(move || {
    //     let mut app = App::new()
    //         .wrap(
    //             Logger::new(
    //                 format!(
    //                     "%{{ERROR_STATUS}}xo %s{}%U %Dms",
    //                     if config.logging.log_ips {
    //                         " %{r}a"
    //                     } else {
    //                         " "
    //                     }
    //                 )
    //                 .as_str(),
    //             )
    //             .custom_response_replace("ERROR_STATUS", log_err)
    //             .log_target("shoelace::web"),
    //         )
    //         .app_data(data.clone())
    //         .default_service(web::to(move || {
    //             common::error::not_found(config.endpoint.frontend)
    //         }))
    //         .service(web::scope("/proxy").service(proxy::serve));

    //     if config.endpoint.frontend {
    //         // Loads static files
    //         let generated = generate();

    //         // Adds services to app
    //         app = app
    //             .service(ResourceFiles::new("/static", generated))
    //             .service(frontend::routes::user)
    //             .service(frontend::routes::post)
    //             .service(frontend::routes::home)
    //             .service(frontend::routes::find)
    //             .service(frontend::routes::redirect);
    //     }

    //     // API
    //     if config.endpoint.api {
    //         app = app.service(web::scope("/api").service(api::post).service(api::user));
    //     }

    //     // RSS
    //     if config.endpoint.rss {
    //         app = app.service(web::scope("/rss").service(rss::user))
    //     }

    //     // Returns app definition
    //     app
    // });

    let tls_params = match config.server.tls {
        Some(opt) => {
            info!("TLS has been enabled");
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
        axum_server::bind(
            format!("{}:{}", config.server.listen, config.server.port)
                .parse()
                .unwrap(),
        )
        .serve(app.into_make_service())
        .await?
    } else {
        let tls_config = RustlsConfig::from_pem_file(tls_params.cert, tls_params.key)
            .await
            .unwrap();

        axum_server::bind_rustls(
            format!("{}:{}", config.server.listen, config.server.port)
                .parse()
                .unwrap(),
            tls_config,
        )
        .serve(app.into_make_service())
        .await?
    };

    info!("🚪 Shoelace exited successfully. See you soon!");
    Ok(())
}
