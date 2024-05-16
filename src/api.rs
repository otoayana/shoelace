use crate::{proxy, req};
use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    web::{self, Data},
    Responder, Result,
};

/// User endpoint
#[get("/user")]
async fn user(form: web::Form<req::UserData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let resp = req::user(form.into_inner(), store).await.map_err(|_| ErrorInternalServerError("request failed"))?;
    
    if let Some(user) = resp {
	Ok(web::Json(user))
    } else {
	Err(ErrorNotFound("null"))
    }
}

/// User endpoint
#[get("/post")]
async fn post(form: web::Form<req::PostData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let resp = req::post(form.into_inner(), store).await.map_err(|_| ErrorInternalServerError("request failed"))?;
    
    if let Some(post) = resp {
	Ok(web::Json(post))
    } else {
	Err(ErrorNotFound("null"))
    }
}
