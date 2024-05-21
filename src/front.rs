use crate::{error::ShoelaceError, proxy, req, TEMPLATES};
use actix_web::{
    get,
    web::{self, Data, Redirect},
    HttpResponse, Responder, ResponseError,
};
use serde::{Deserialize, Serialize};
use spools::{Post, User};
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};
use tera::Context;

/// For user template
#[derive(Debug, Deserialize, Serialize)]
struct UserResponse {
    request: String,
    response: User,
    time: u128,
}

/// For post template
#[derive(Debug, Deserialize, Serialize)]
struct PostResponse {
    response: Post,
    time: u128,
}

/// Used by the find form
#[derive(Debug, Deserialize)]
struct Find {
    value: String,
}

/// Logs how much it takes for a request to finish
fn time_log() -> Result<u128, SystemTimeError> {
    let start = SystemTime::now();

    let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_millis();

    Ok(since_the_epoch)
}

/// Home
#[get("/")]
async fn home() -> Result<HttpResponse, ShoelaceError> {
    let resp = TEMPLATES.render("home.html", &Context::new())?;

    Ok(HttpResponse::Ok().body(resp))
}

/// User frontend
#[get("/@{user}")]
async fn user(
    user: web::Path<String>,
    store: Data<proxy::KeyStore>,
) -> Result<HttpResponse, ShoelaceError> {
    let start_time = time_log()?;

    let req = req::user(req::UserData { tag: user.clone() }, store).await?;

    let end_time = time_log()?;

    let response_time = end_time - start_time;

    let data = UserResponse {
        request: user.into_inner(),
        response: req,
        time: response_time,
    };

    let resp = TEMPLATES.render("user.html", &Context::from_serialize(data)?)?;

    Ok(HttpResponse::Ok().body(resp))
}

/// Post frontend
#[get("/t/{post}")]
async fn post(
    post: web::Path<String>,
    store: Data<proxy::KeyStore>,
) -> Result<HttpResponse, ShoelaceError> {
    let start_time = time_log()?;

    let req = req::post(
        req::PostData {
            id: post.into_inner(),
        },
        store,
    )
    .await?;

    let end_time = time_log()?;

    let total_time = end_time - start_time;

    let resp = crate::TEMPLATES.render(
        "post.html",
        &Context::from_serialize(PostResponse {
            response: req,
            time: total_time,
        })?,
    )?;

    Ok(HttpResponse::Ok().body(resp))
}

/// User finder endpoint
#[get("/find")]
async fn find(request: web::Query<Find>) -> impl Responder {
    let values = request.into_inner();

    Redirect::to(format!("/@{}", values.value)).temporary()
}

/// Post redirect endpoint
#[get("/{_}/post/{path}")]
async fn redirect(request: web::Path<((), String)>) -> impl Responder {
    let values = request.into_inner();

    Redirect::to(format!("/t/{}", values.1)).permanent()
}