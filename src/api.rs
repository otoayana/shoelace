use crate::{req, Error, ShoelaceData};
use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    web::{self, Data},
    Responder, Result,
};
use spools::SpoolsError;

/// User API endpoint
#[get("/user")]
async fn user(form: web::Form<req::UserData>, store: Data<ShoelaceData>) -> Result<impl Responder> {
    // Fetch user
    let resp = req::user(form.into_inner(), store).await;

    // We need to unwrap this error as such, since we don't want to return a fully rendered HTML page on an API.
    match resp {
        Ok(body) => Ok(web::Json(body)),
        Err(error) => match error {
            Error::ThreadsError(spools_err) => {
                if let SpoolsError::NotFound(_) = spools_err {
                    Err(ErrorNotFound(spools_err))
                } else {
                    Err(ErrorInternalServerError(spools_err))
                }
            }
            _ => Err(ErrorInternalServerError(error)),
        },
    }
}

/// Post API endpoint
#[get("/post")]
async fn post(form: web::Form<req::PostData>, store: Data<ShoelaceData>) -> Result<impl Responder> {
    let resp = req::post(form.into_inner(), store).await;

    // We need to unwrap this error as such, since we don't want to return a fully rendered HTML page on an API.
    match resp {
        Ok(body) => Ok(web::Json(body)),
        Err(error) => match error {
            Error::ThreadsError(spools_err) => {
                if let SpoolsError::NotFound(_) = spools_err {
                    Err(ErrorNotFound(spools_err))
                } else {
                    Err(ErrorInternalServerError(spools_err))
                }
            }
            _ => Err(ErrorInternalServerError(error)),
        },
    }
}
