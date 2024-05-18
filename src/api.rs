use crate::{proxy, req};
use actix_web::{
    error::ErrorInternalServerError,
    get,
    web::{self, Data},
    Responder, Result,
};

/// User endpoint
#[get("/user")]
async fn user(
    form: web::Form<req::UserData>,
    store: Data<proxy::KeyStore>,
) -> Result<impl Responder> {
    let resp = req::user(form.into_inner(), store)
        .await
        .map_err(|_| ErrorInternalServerError("request failed"))?;

    Ok(web::Json(resp))
}

/// Post endpoint
#[get("/post")]
async fn post(
    form: web::Form<req::PostData>,
    store: Data<proxy::KeyStore>,
) -> Result<impl Responder> {
    let resp = req::post(form.into_inner(), store)
        .await
        .map_err(|_| ErrorInternalServerError("request failed"))?;

    Ok(web::Json(resp))
}
