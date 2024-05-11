use crate::{proxy, scraping};
use actix_web::{
    get,
    web::{self, Data},
    Responder, Result,
};
use serde::Deserialize;
use std::thread;

#[derive(Deserialize)]
struct UserData {
    tag: String,
}

#[derive(Deserialize)]
struct PostData {
    tag: String,
    id: String,
}

#[get("/user")]
async fn user(form: web::Form<UserData>, store: Data<proxy::Db>) -> Result<impl Responder> {
    let user = thread::spawn(move || scraping::user(&form.tag, Some(store)))
        .join()
        .expect("Thread panicked");
    Ok(web::Json(user.unwrap_or_default()))
}

#[get("/post")]
async fn post(form: web::Form<PostData>, store: Data<proxy::Db>) -> Result<impl Responder> {
    let post = thread::spawn(move || scraping::post(&form.tag, &form.id, Some(store)))
        .join()
        .expect("Thread panicked");
    Ok(web::Json(post.unwrap_or_default()))
}
