use crate::{proxy, req, TEMPLATES};
use actix_web::{
    error::ErrorInternalServerError,
    get,
    web::{self, Data},
    HttpResponse,
};
use serde::{Deserialize, Serialize};
use spools::{Post, User};
use tera::Context;

#[derive(Debug, Deserialize, Serialize)]
struct UserResponse {
    request: String,
    response: User,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostResponse {
    response: Post,
}

/// User endpoint
#[get("/@{user}")]
async fn user(user: web::Path<String>, store: Data<proxy::KeyStore>) -> HttpResponse {
    let req = req::user(req::UserData { tag: user.clone() }, store)
        .await
        .map_err(|_| ErrorInternalServerError("request failed"))
        .unwrap();

    let data = UserResponse {
        request: user.into_inner(),
        response: req,
    };

    let resp = TEMPLATES.render("user.html", &Context::from_serialize(data).unwrap());

    match resp {
        Ok(body) => HttpResponse::Ok().body(body),
        Err(body) => HttpResponse::InternalServerError().body(body.to_string()),
    }
}

/// Post endpoint
#[get("/t/{post}")]
async fn post(post: web::Path<String>, store: Data<proxy::KeyStore>) -> HttpResponse {
    let req = req::post(
        req::PostData {
            id: post.into_inner(),
        },
        store,
    )
    .await
    .map_err(|_| ErrorInternalServerError("request failed"))
    .unwrap();

    let resp = crate::TEMPLATES
        .render(
            "post.html",
            &Context::from_serialize(PostResponse { response: req }).map_err(|x| actix_web::error::ErrorInternalServerError(format!("response error: {}", x))).unwrap(),
        )
        .map_err(|x| actix_web::error::ErrorInternalServerError(format!("could not render template: {}", x)));

    match resp {
        Ok(body) => HttpResponse::Ok().body(body),
        Err(body) => HttpResponse::InternalServerError().body(body.to_string()),
    }
}
