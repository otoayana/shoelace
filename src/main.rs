#[macro_use]
extern crate lazy_static;

mod api;
mod error;
mod front;
mod proxy;
mod req;

use actix_files::Files;
use actix_web::{
    middleware::{Compat, Logger},
    web, App, HttpServer,
};
use proxy::KeyStore;
use std::collections::HashMap;
use tera::Tera;
use tokio::sync::Mutex;
use tracing_actix_web::TracingLogger;

// Import templates
lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let tera = match Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")) {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera
    };
}

/// Web server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logger
    tracing_subscriber::fmt::init();

    // initialize keystore
    let db = web::Data::new(KeyStore {
        content: Mutex::new(HashMap::new()),
    });

    // Start up web server
    HttpServer::new(move || {
        App::new()
            .wrap(Compat::new(TracingLogger::default()))
            .wrap(Logger::default())
            .service(Files::new(
                "/static",
                concat!(env!("CARGO_MANIFEST_DIR"), "/static"),
            ))
            .service(front::user)
            .service(front::post)
            .service(front::home)
            .service(front::find)
            .service(front::redirect)
            .service(web::scope("/api/v1").service(api::post).service(api::user))
            .service(web::scope("/proxy").service(proxy::proxy))
	    .default_service(web::to(|| error::not_found()))
            .app_data(db.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
