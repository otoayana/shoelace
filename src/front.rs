use std::time::{SystemTime, UNIX_EPOCH};

use crate::{proxy, req, TEMPLATES};
use actix_web::{
    error::ErrorInternalServerError,
    get,
    web::{self, Data, Redirect},
    HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};
use spools::{Post, User};
use tera::Context;

#[derive(Debug, Deserialize, Serialize)]
struct UserResponse {
    request: String,
    response: User,
    time: u128,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostResponse {
    response: Post,
    time: u128,
}

#[derive(Debug, Deserialize)]
struct FormRedirect {
    value: String,
}

fn time_log() -> anyhow::Result<u128> {
    let start = SystemTime::now();

    let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_millis();

    Ok(since_the_epoch)
}

/// Homepage
#[get("/")]
async fn home() -> HttpResponse {
    let resp = TEMPLATES.render("home.html", &Context::new()).map_err(|x| {
        actix_web::error::ErrorInternalServerError(format!("could not render template: {}", x))
    });

    match resp {
        Ok(body) => HttpResponse::Ok().body(body),
        Err(body) => HttpResponse::InternalServerError().body(body.to_string()),
    }
}

/// User endpoint
#[get("/@{user}")]
async fn user(user: web::Path<String>, store: Data<proxy::KeyStore>) -> HttpResponse {
    let start_time = time_log()
        .map_err(|_| ErrorInternalServerError("coudln't fetch time"))
        .unwrap();

    let req = req::user(req::UserData { tag: user.clone() }, store)
        .await
        .map_err(|_| ErrorInternalServerError("request failed"))
        .unwrap();

    let end_time = time_log()
        .map_err(|_| ErrorInternalServerError("coudln't fetch time"))
        .unwrap();

    let response_time = end_time - start_time;

    let data = UserResponse {
        request: user.into_inner(),
        response: req,
        time: response_time,
    };

    let resp = TEMPLATES.render(
        "user.html",
        &Context::from_serialize(data)
            .map_err(|x| {
                actix_web::error::ErrorInternalServerError(format!("response error: {}", x))
            })
            .unwrap(),
    );

    match resp {
        Ok(body) => HttpResponse::Ok().body(body),
        Err(body) => HttpResponse::InternalServerError().body(body.to_string()),
    }
}

/// Post endpoint
#[get("/t/{post}")]
async fn post(post: web::Path<String>, store: Data<proxy::KeyStore>) -> HttpResponse {
    let start_time = time_log()
        .map_err(|_| ErrorInternalServerError("coudln't fetch time"))
        .unwrap();

    let req = req::post(
        req::PostData {
            id: post.into_inner(),
        },
        store,
    )
    .await
    .map_err(|_| ErrorInternalServerError("request failed"))
    .unwrap();

    let end_time = time_log()
        .map_err(|_| ErrorInternalServerError("coudln't fetch time"))
        .unwrap();

    let total_time = end_time - start_time;

    let resp = crate::TEMPLATES
        .render(
            "post.html",
            &Context::from_serialize(PostResponse {
                response: req,
                time: total_time,
            })
            .map_err(|x| {
                actix_web::error::ErrorInternalServerError(format!("response error: {}", x))
            })
            .unwrap(),
        )
        .map_err(|x| {
            actix_web::error::ErrorInternalServerError(format!("could not render template: {}", x))
        });

    match resp {
        Ok(body) => HttpResponse::Ok().body(body),
        Err(body) => HttpResponse::InternalServerError().body(body.to_string()),
    }
}

/// Redirect endpoint
#[get("/redirect")]
async fn redirect(request: web::Query<FormRedirect>) -> impl Responder {
    let values = request.into_inner();

    Redirect::to(format!("/@{}", values.value)).temporary()
}
