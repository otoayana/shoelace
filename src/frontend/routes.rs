use crate::{
    frontend::templates::{Base, HomeView, PostView, UserView},
    req, Error, ShoelaceData,
};
use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data, Redirect},
    HttpResponse, Responder,
};
use askama::Template;
use serde::{Deserialize, Serialize};
use spools::{Post, User};

#[derive(Debug, Deserialize, Serialize)]
enum ResponseTypes {
    Post(Post),
    User(User),
}

// For user template
#[derive(Debug, Deserialize, Serialize)]
struct Response {
    request: String,
    response: ResponseTypes,
    time: u128,
    rev: String,
    base_url: String,
    rss: bool,
}

// For user find form
#[derive(Debug, Deserialize)]
struct Find {
    value: String,
}

// Landing page
#[get("/")]
async fn home() -> Result<HttpResponse, Error> {
    let base = Base::new()?;
    let template = HomeView { base }.render()?;

    Ok(HttpResponse::Ok()
        .insert_header(ContentType::html())
        .body(template))
}

// User frontend
#[get("/@{user}")]
async fn user(user: web::Path<String>, store: Data<ShoelaceData>) -> Result<HttpResponse, Error> {
    let mut base = Base::new()?;

    base.timer(true)?;
    let req = req::user(user.clone(), store.to_owned()).await?;
    base.timer(false)?;

    let template = UserView {
        base,
        input: &user.into_inner(),
        output: req,
    }
    .render()?;

    Ok(HttpResponse::Ok()
        .insert_header(ContentType::html())
        .body(template))
}

// Post frontend
#[get("/t/{post}")]
async fn post(post: web::Path<String>, store: Data<ShoelaceData>) -> Result<HttpResponse, Error> {
    let mut base = Base::new()?;

    base.timer(true)?;
    let req = req::post(post.clone(), store.to_owned()).await?;
    base.timer(false)?;

    let template = PostView {
        base,
        input: &post.into_inner(),
        output: req,
    }
    .render()?;

    Ok(HttpResponse::Ok()
        .insert_header(ContentType::html())
        .body(template))
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
