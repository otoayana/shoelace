#[macro_use]
extern crate lazy_static;

// Defines crate modules and re-exports
mod api;
mod common;
mod front;
mod proxy;
mod rss;

pub(crate) use common::config;
pub(crate) use common::error::Error;
pub(crate) use common::req;
use tracing_log::LogTracer;

#[cfg(test)]
mod test;

// Main application begins here
use crate::common::config::{Settings, Tls};
use actix_web::{dev::ServiceResponse, middleware::Logger, web, App, HttpServer};
use actix_web_static_files::ResourceFiles;
use git_version::git_version;
use include_dir::{include_dir, Dir};
use proxy::Keystore;
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind},
    process::id,
    sync::Mutex,
};
use tera::Tera;
use tracing::{info, instrument, warn};
use tracing_subscriber::{fmt::Layer, prelude::*, EnvFilter, Registry};

// Define application data
#[derive(Debug)]
pub(crate) struct ShoelaceData {
    pub(crate) store: Keystore,
    pub(crate) log_cdn: bool,
    pub(crate) base_url: String,
    pub(crate) rev: String,
    pub(crate) rss: bool,
}

// Bundle in folders on compile time
pub(crate) static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

// Import templates
lazy_static! {
    pub(crate) static ref TEMPLATES: Tera = {
        let mut tera = Tera::default();

        // Fetches templates from included template directory
        let templates: Vec<(&str, &str)> = TEMPLATES_DIR
            .find("**/*.html")
            .expect("Templates not found")
            .map(|file| {
                (
                    file.path().to_str().unwrap_or(""),
                    file.as_file()
                        .expect("Not a file")
                        .contents_utf8()
                        .unwrap_or(""),
                )
            })
            .collect::<Vec<(&str, &str)>>();

        // Adds them to our Tera variable
        match tera.add_raw_templates(templates) {
            Ok(_) => tera,
            Err(error) => {
                eprintln!("Parsing error(s): {}", error);
                ::std::process::exit(1)
            }
        }
    };
}

// Sets characters depending on web server response code
fn log_err(res: &ServiceResponse) -> String {
    let status = res.status().as_u16();

    if status == 404 {
        "??".to_string()
    } else if status >= 400 {
        "!!".to_string()
    } else {
        "<3".to_string()
    }
}

// Web server
#[instrument(name = "shoelace::main")]
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Parses config
    let config = Settings::new().map_err(|err| io::Error::new(ErrorKind::InvalidInput, err))?;

    // Create log filter, in order to exclude Actix's verbose logs, due to many of them being too verbose to be useful.
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

    // Initialize logging registry
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

    // Create subscriber
    tracing::subscriber::set_global_default(registry).unwrap();

    // Adapt logs from `log` crate into tracing logs
    LogTracer::init().map_err(|err| io::Error::new(ErrorKind::Other, err))?;

    // Fetch revision
    let rev = git_version!(
        args = ["--always", "--dirty=-dirty"],
        fallback = format!("v{}", env!("CARGO_PKG_VERSION"))
    )
    .to_string();

    // Startup message
    info!(
        "👟 Shoelace {} | PID: {} | https://sr.ht/~nixgoat/shoelace",
        &rev,
        id()
    );

    // Assigns application data
    let data = web::Data::new(ShoelaceData {
        // Proxy backends
        store: Keystore::new(config.proxy)
            .await
            .map_err(|err| io::Error::new(ErrorKind::ConnectionRefused, err))?,
        // CDN logging setting
        log_cdn: config.logging.log_cdn,
        // Base URL
        base_url: config.server.base_url.clone(),
        // Git/Cargo revision
        rev,
        // RSS enabled (for displaying button in FE)
        rss: config.endpoint.rss,
    });

    // Notify the admin about what base URL was stuff
    info!("Base URL is set to {}", config.server.base_url);

    // Notify administrator if any endpoints are disabled
    if !config.endpoint.frontend {
        warn!("Frontend has been disabled");
    }

    if !config.endpoint.api {
        warn!("API has been disabled");
    }

    // Configures web server
    let mut server = HttpServer::new(move || {
        // Defines app base
        let mut app = App::new()
            // Start web request logger
            .wrap(
                Logger::new(
                    format!(
                        "%{{ERROR_STATUS}}xo %s{}%U %Dms",
                        if config.logging.log_ips {
                            " %{r}a"
                        } else {
                            " "
                        }
                    )
                    .as_str(),
                )
                .custom_response_replace("ERROR_STATUS", log_err)
                .log_target("shoelace::web"),
            )
            .app_data(data.clone())
            /* Set 404 page to be the default page shown if no routes are provided.
            If the frontend is displayed, these will be replaced by a plaintext version.*/
            .default_service(web::to(move || {
                common::error::not_found(config.endpoint.frontend)
            }))
            .service(web::scope("/proxy").service(proxy::serve));

        // Frontend
        if config.endpoint.frontend {
            // Loads static files
            let generated = generate();

            // Adds services to app
            app = app
                .service(ResourceFiles::new("/static", generated))
                .service(front::user)
                .service(front::post)
                .service(front::home)
                .service(front::find)
                .service(front::redirect);
        }

        // API
        if config.endpoint.api {
            app = app.service(web::scope("/api").service(api::post).service(api::user));
        }

        // RSS
        if config.endpoint.rss {
            app = app.service(web::scope("/rss").service(rss::user))
        }

        // Returns app definition
        app
    });

    // Checks if there's any TLS settings
    let tls_params = match config.server.tls {
        Some(opt) => opt,
        None => Tls {
            enabled: false,
            cert: String::new(),
            key: String::new(),
        },
    };

    // TLS
    if tls_params.enabled {
        // Loads certificate chain file
        let mut certs_file = BufReader::new(File::open(tls_params.cert)?);

        // Loads key file
        let mut key_file = BufReader::new(File::open(tls_params.key)?);

        // Loads certificates
        let tls_certs = rustls_pemfile::certs(&mut certs_file).collect::<Result<Vec<_>, _>>()?;

        // Loads key
        let tls_key = match rustls_pemfile::pkcs8_private_keys(&mut key_file).next() {
            Some(key) => key?,
            None => return Err(io::Error::new(ErrorKind::InvalidInput, "Not a key file")),
        };

        // Configures server using provided files
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
            .map_err(|err| {
                io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Could not configure TLS server: {}", err),
                )
            })?;

        // Binds server with TLS
        server = server.bind_rustls_0_23(
            (config.server.listen.clone(), config.server.port),
            tls_config,
        )?;

        info!("TLS has been enabled");
    } else {
        // Binds server without TLS
        server = server.bind((config.server.listen.clone(), config.server.port))?;
    }

    // Now that everything is configured, notify the admin the server is up!
    info!(
        "Accepting connections at {}:{}",
        config.server.listen, config.server.port
    );

    // Runs server
    let run = server.run().await;

    // Notify whenever the server stops
    info!("🚪 Shoelace exited successfully. See you soon!");
    run
}
