use crate::{error::ShoelaceError, proxy, req};
use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    web::{self, Data},
    Responder, Result,
};
use spools::SpoolsError;

/// User API endpoint
#[get("/user")]
async fn user(
    form: web::Form<req::UserData>,
    store: Data<proxy::KeyStore>,
) -> Result<impl Responder> {
    // Fetch user
    let resp = req::user(form.into_inner(), store).await;

    // We need to unwrap this error as such, since we don't want to return a fully rendered HTML page on an API.
    if let Err(error) = resp {
        match error {
            ShoelaceError::ThreadsError(spools_err) => {
                if let SpoolsError::NotFound(_) = spools_err {
                    Ok(Err(ErrorNotFound(spools_err)))
                } else {
                    Ok(Err(ErrorInternalServerError(spools_err)))
                }
            }
            _ => Err(ErrorInternalServerError(error)),
        }
    } else {
        Ok(Ok(web::Json(resp.unwrap())))
    }
}

/// Post API endpoint
#[get("/post")]
async fn post(
    form: web::Form<req::PostData>,
    store: Data<proxy::KeyStore>,
) -> Result<impl Responder> {
    let resp = req::post(form.into_inner(), store).await;

    // We need to unwrap this error as such, since we don't want to return a fully rendered HTML page on an API.
    if let Err(error) = resp {
        match error {
            ShoelaceError::ThreadsError(spools_err) => {
                if let SpoolsError::NotFound(_) = spools_err {
                    Ok(Err(ErrorNotFound(spools_err)))
                } else {
                    Ok(Err(ErrorInternalServerError(spools_err)))
                }
            }
            _ => Err(ErrorInternalServerError(error)),
        }
    } else {
        Ok(Ok(web::Json(resp.unwrap())))
    }
}
