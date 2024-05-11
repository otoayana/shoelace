mod api;
mod proxy;
mod scraping;
mod utils;
use std::collections::HashMap;

use actix_web::{web, App, HttpServer};
use proxy::Db;
use tokio::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = web::Data::new(Db {
        content: Mutex::new(HashMap::new()),
    });
    HttpServer::new(move || {
        App::new()
            .service(web::scope("/api/v1").service(api::post).service(api::user))
            .service(web::scope("/proxy").service(proxy::proxy))
            .app_data(db.clone())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
