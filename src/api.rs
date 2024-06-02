use crate::{req, ShoelaceData};
use actix_web::{
    get,
    web::{self, Data},
    Responder, Result,
};
// User API endpoint
#[get("/user/{user}")]
async fn user(path: web::Path<String>, store: Data<ShoelaceData>) -> Result<impl Responder> {
    // Fetch user
    let resp = req::user(path.into_inner(), store).await;

    match resp {
        Ok(body) => Ok(web::Json(body)),
        Err(error) => Err(error.to_plaintext()),
    }
}

// Post API endpoint
#[get("/post/{id}")]
async fn post(path: web::Path<String>, store: Data<ShoelaceData>) -> Result<impl Responder> {
    let resp = req::post(path.into_inner(), store).await;

    match resp {
        Ok(body) => Ok(web::Json(body)),
        Err(error) => Err(error.to_plaintext()),
    }
}
