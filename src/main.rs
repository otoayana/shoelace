#[macro_use]
extern crate lazy_static;

mod api;
mod config;
mod error;
mod front;
mod proxy;
mod req;

use actix_web::{
    middleware::{Compat, Logger},
    web, App, HttpServer,
};
use actix_web_static_files::ResourceFiles;
use config::{ProxyModes, Settings};
use include_dir::{include_dir, Dir};
use std::{collections::HashMap, fs::File, io::BufReader};
use tera::Tera;
use tokio::sync::Mutex;
use tracing_actix_web::TracingLogger;

// Define application data
#[allow(unused)]
pub(crate) struct ShoelaceData {
    pub(crate) store: Option<KeyStore>,
    pub(crate) base_url: String,
}

// Define keystores
pub(crate) enum KeyStore {
    Internal(Mutex<HashMap<String, String>>),
    RocksDB(rocksdb::DB),
    Redis(redis::aio::MultiplexedConnection),
}

// Implement graceful shutdown
impl Drop for KeyStore {
    fn drop(&mut self) {
        // Only RocksDB needs it, in order to close unfinished connections before shutting down
        if let Self::RocksDB(val) = self {
            val.cancel_all_background_work(true)
        }
    }
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

/// Web server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initializes logger
    tracing_subscriber::fmt::init();

    // Parses config
    let maybe_config = config::Settings::new();
    let config: Settings;

    // Check whether the user has provided a config file
    if let Ok(got_config) = maybe_config {
        config = got_config
    } else {
        return Err(maybe_config
            .map_err(|x| std::io::Error::new(std::io::ErrorKind::InvalidInput, x))
            .unwrap_err());
    }

    // Defines application data
    let data = web::Data::new(ShoelaceData {
        // Proxy backends
        store: match &config.proxy.backend {
            // RocksDB
            ProxyModes::RocksDB => Some(KeyStore::RocksDB(
                // Open keystore in the provided path
                rocksdb::DB::open_default(config.proxy.rocksdb.unwrap().path)
                    .expect("couldn't open database"),
            )),
            // Redis
            ProxyModes::Redis => Some(KeyStore::Redis({
                // Configure client using the URI provided by the user
                let client = redis::Client::open(config.proxy.redis.unwrap().uri).unwrap();

                // Establish connection
                client
                    .get_multiplexed_async_connection()
                    .await
                    .expect("couldn't connect to redis")
            })),
            // Internal (Creates hash map)
            ProxyModes::Internal => Some(KeyStore::Internal(Mutex::new(HashMap::new()))),
            // None (Sets Option as None)
            ProxyModes::None => None,
        },
        // Base URL
        base_url: config.server.base_url,
    });

    // Configures web server
    let mut server = HttpServer::new(move || {
        // Defines app base
        let mut app = App::new()
            .wrap(Compat::new(TracingLogger::default()))
            .wrap(Logger::default())
            .default_service(web::to(move || error::not_found(config.endpoint.frontend)))
            .app_data(data.clone());

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

        // Proxy (If enabled)
        if !matches!(config.proxy.backend, ProxyModes::None) {
            app = app.service(web::scope("/proxy").service(proxy::proxy));
        }

        // Returns app definition
        app
    });

    // TLS
    if config.server.tls.enabled {
        // Loads certificate chain file
        let mut certs_file = BufReader::new(
            File::open(config.server.tls.cert).expect("Unable to open certficate file"),
        );

        // Loads key file
        let mut key_file = BufReader::new(
            File::open(config.server.tls.key).expect("Unable to open certficate file"),
        );

        // Loads certificates
        let tls_certs = rustls_pemfile::certs(&mut certs_file)
            .collect::<Result<Vec<_>, _>>()
            .expect("Not a certificate chain");

        // Loads key
        let tls_key = rustls_pemfile::pkcs8_private_keys(&mut key_file)
            .next()
            .expect("Not a key file")
            .expect("Not a key file");

        // Configures server using provided files
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
            .expect("Unable to configure TLS");

        // Binds server with TLS
        server = server.bind_rustls_0_22((config.server.listen, config.server.port), tls_config)?;
    } else {
        // Binds server without TLS
        server = server.bind((config.server.listen, config.server.port))?;
    }

    // Runs server
    server.run().await
}
