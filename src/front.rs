use crate::{proxy, req, TEMPLATES};
use actix_web::{
    error::ErrorInternalServerError,
    get,
    web::{self, Data},
    HttpResponse,
};
use tera::Context;

/// User endpoint
#[get("/@{user}")]
async fn user(user: web::Path<String>, store: Data<proxy::KeyStore>) -> HttpResponse {
    let req = req::user(
        req::UserData {
            tag: user.into_inner(),
        },
        store,
    )
    .await
    .map_err(|_| ErrorInternalServerError("request failed"))
    .unwrap();

    if let Some(user) = req {
        dbg!(&Context::from_serialize(&user).unwrap());
        let resp = TEMPLATES.render("user.html", &Context::from_serialize(user).unwrap());

        match resp {
            Ok(body) => HttpResponse::Ok().body(body),
            Err(body) => HttpResponse::InternalServerError().body(body.to_string()),
        }
    } else {
        HttpResponse::NotFound().body("not found")
    }
}
