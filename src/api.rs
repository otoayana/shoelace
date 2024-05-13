use crate::{proxy, scraping};
use actix_web::{
    get,
    web::{self, Data},
    Responder, Result,
};
use serde::Deserialize;
use std::thread;

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

#[get("/user")]
async fn user(form: web::Form<UserData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let user = thread::spawn(move || scraping::user(&form.tag, Some(store)))
        .join()
        .expect("Thread panicked");
    Ok(web::Json(user.unwrap_or_default()))
}

#[get("/post")]
async fn post(form: web::Form<PostData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let post = thread::spawn(move || scraping::post(&form.id, Some(store)))
        .join()
        .expect("Thread panicked");
    Ok(web::Json(post.unwrap_or_default()))
}
