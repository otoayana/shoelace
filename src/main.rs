#[macro_use]
extern crate lazy_static;

// Defines crate modules and re-exports
mod api;
mod common;
mod front;
mod proxy;

pub(crate) use common::config;
pub(crate) use common::error::Error;
pub(crate) use common::req;

// Application begins here
use crate::common::config::{Settings, Tls};
use actix_web::{
    dev::ServiceResponse,
    middleware::{Compat, Logger},
    web, App, HttpServer,
};
use actix_web_static_files::ResourceFiles;
use include_dir::{include_dir, Dir};
use proxy::Keystore;
use std::{
    fs::File,
    io::{self, BufReader, ErrorKind},
};
use tera::Tera;
use tracing_actix_web::TracingLogger;

// Define application data
#[allow(unused)]
pub(crate) struct ShoelaceData {
    pub(crate) store: Keystore,
    pub(crate) base_url: String,
}

// Bundle in folders on compile time
pub static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates");
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

// Import templates
lazy_static! {
    pub static ref TEMPLATES: Tera = {
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

fn logerr(res: &ServiceResponse) -> String {
    let status = res.status().as_u16();

    if status == 404 {
        "??".to_string()
    } else if status >= 400 {
        "!!".to_string()
    } else {
        "<3".to_string()
    }
}

/// Web server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initializes logger
    tracing_subscriber::fmt::init();

    // Parses config
    let config = Settings::new().map_err(|err| io::Error::new(ErrorKind::InvalidInput, err))?;

    // Assigns application data
    let data = web::Data::new(ShoelaceData {
        // Proxy backends
        store: Keystore::new(config.proxy)
            .await
            .map_err(|err| io::Error::new(ErrorKind::ConnectionRefused, err))?,
        // Base URL
        base_url: config.server.base_url,
    });

    // Configures web server
    let mut server = HttpServer::new(move || {
        // Defines app base
        let mut app = App::new()
            .wrap(Compat::new(TracingLogger::default()))
            .wrap(
                Logger::new("%{ERROR_STATUS}xo %s %U %sms")
                    .custom_response_replace("ERROR_STATUS", |res| logerr(res))
                    .log_target("shoelace::web"),
            )
            .app_data(data.clone())
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
                .service(front::redirect)
        }

        // API
        if config.endpoint.api {
            app = app.service(web::scope("/api/v1").service(api::post).service(api::user))
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
        server = server.bind_rustls_0_22((config.server.listen, config.server.port), tls_config)?;
    } else {
        // Binds server without TLS
        server = server.bind((config.server.listen, config.server.port))?;
    }

    // Runs server
    server.run().await
}
