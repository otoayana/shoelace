use crate::{req, Error, ShoelaceData, TEMPLATES};
use actix_web::{
    get,
    web::{self, Data, Redirect},
    HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use spools::{Post, User};
use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};
use tera::Context;

// For user template
#[derive(Debug, Deserialize, Serialize)]
struct UserResponse {
    request: String,
    response: User,
    time: u128,
    rev: String,
}

// For post template
#[derive(Debug, Deserialize, Serialize)]
struct PostResponse {
    response: Post,
    time: u128,
    rev: String,
}

// For user find form
#[derive(Debug, Deserialize)]
struct Find {
    value: String,
}

// Logs current time, in order to determine request times
fn time_log() -> Result<u128, SystemTimeError> {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_millis();

    Ok(since_the_epoch)
}

// Landing page
#[get("/")]
async fn home(store: Data<ShoelaceData>) -> Result<HttpResponse, Error> {
    let mut context = Context::new();
    context.insert("rev", &store.rev);
    let resp = TEMPLATES.render("home.html", &context)?;

    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(resp))
}

// User frontend
#[get("/@{user}")]
async fn user(user: web::Path<String>, store: Data<ShoelaceData>) -> Result<HttpResponse, Error> {
    // Get current timestamp before request
    let start_time = time_log()?;

    // Process user request
    let req = req::user(req::UserData { tag: user.clone() }, store.to_owned()).await?;

    // Get request time
    let end_time = time_log()?;
    let response_time = end_time - start_time;

    // Define response values
    let data = UserResponse {
        request: user.into_inner(),
        response: req,
        time: response_time,
        rev: store.rev.clone(),
    };

    // Render template from those values
    let resp = TEMPLATES.render("user.html", &Context::from_serialize(data)?)?;

    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(resp))
}

// Post frontend
#[get("/t/{post}")]
async fn post(post: web::Path<String>, store: Data<ShoelaceData>) -> Result<HttpResponse, Error> {
    // Get current timestamp before request
    let start_time = time_log()?;

    // Process post request
    let req = req::post(
        req::PostData {
            id: post.into_inner(),
        },
        store.to_owned(),
    )
    .await?;

    // Get request time
    let end_time = time_log()?;
    let total_time = end_time - start_time;

    // Define response values
    let data = PostResponse {
        response: req,
        time: total_time,
        rev: store.rev.clone(),
    };

    // Render template from those values
    let resp = crate::TEMPLATES.render("post.html", &Context::from_serialize(data)?)?;

    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(resp))
}

// User finder endpoint
#[get("/find")]
async fn find(request: web::Query<Find>) -> impl Responder {
    // Fetches query value
    let values = request.into_inner();

    // Redirects to user
    Redirect::to(format!("/@{}", values.value)).temporary()
}

// Post redirect endpoint
#[get("/{_}/post/{path}")]
async fn redirect(request: web::Path<((), String)>) -> impl Responder {
    // Fetches path values
    let values = request.into_inner();

    // Redirects to post
    Redirect::to(format!("/t/{}", values.1)).permanent()
}
