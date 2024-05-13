mod api;
mod proxy;
mod scraping;
mod utils;

use actix_web::{middleware::{Compat, Logger}, web, App, HttpServer};
use proxy::KeyStore;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing_actix_web::TracingLogger;

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
            .service(web::scope("/api/v1").service(api::post).service(api::user))
            .service(web::scope("/proxy").service(proxy::proxy))
            .app_data(db.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
