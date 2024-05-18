#[macro_use]
extern crate lazy_static;

mod api;
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

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let tera = match Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/*")) {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();
    let db = web::Data::new(KeyStore {
        content: Mutex::new(HashMap::new()),
    });
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
            .service(web::scope("/api/v1").service(api::post).service(api::user))
            .service(web::scope("/proxy").service(proxy::proxy))
            .app_data(db.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
