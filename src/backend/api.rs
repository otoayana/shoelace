use crate::backend::{proxy, scraping};
use actix_web::{
    error::ErrorInternalServerError,
    get,
    web::{self, Data},
    Responder, Result,
};
use serde::Deserialize;
use tokio::task;

/// Required values for User endpoint
#[derive(Deserialize)]
struct UserData {
    tag: String,
}

/// Required values for Post endpoint
#[derive(Deserialize)]
struct PostData {
    id: String,
}

/// User endpoint
#[get("/user")]
async fn user(form: web::Form<UserData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let user = task::spawn_blocking(move || scraping::user(&form.tag, Some(store)))
        .await
        .map_err(|_| actix_web::Error::from(ErrorInternalServerError("couldn't fetch user")))?;
    Ok(web::Json(user.unwrap_or_default()))
}

/// Post endpoint
#[get("/post")]
async fn post(form: web::Form<PostData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let post = task::spawn_blocking(move || scraping::post(&form.id, Some(store)))
        .await
        .map_err(|_| actix_web::Error::from(ErrorInternalServerError("couldn't fetch post")))?;
    Ok(web::Json(post.unwrap_or_default()))
}
