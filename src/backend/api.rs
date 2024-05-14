use crate::backend::proxy;
use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    web::{self, Data},
    Responder, Result,
};
use serde::Deserialize;
use spools::Threads;
use tokio::task;

/// Required values for User endpoint
#[derive(Deserialize)]
struct UserData {
    tag: String,
}

/// Required values for Post endpoint
#[derive(Deserialize)]
struct PostData {
    id: String,
}

/// User endpoint
#[get("/user")]
async fn user(form: web::Form<UserData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let resp = task::spawn_blocking(move || {
        let thread = Threads::new()?;
        thread.fetch_user(&form.tag)
    })
    .await
    .map_err(|_| actix_web::Error::from(ErrorInternalServerError("couldn't fetch user")))?;

    if let Some(mut user) = resp.unwrap() {
        let image = task::spawn_blocking(move || {
            proxy::store(user.pfp.unwrap_or(String::new()).as_str(), store)
        })
        .await
        .map_err(|_| ErrorInternalServerError("couldn't spawn thread"))?
        .map_err(|_| ErrorInternalServerError("couldn't store image in proxy"))?;

        user.pfp = Some(image);

        Ok(web::Json(Some(user)))
    } else {
        Err(ErrorNotFound("null"))
    }
}

/// User endpoint
#[get("/post")]
async fn post(form: web::Form<PostData>, store: Data<proxy::KeyStore>) -> Result<impl Responder> {
    let resp = task::spawn_blocking(move || {
        let thread = Threads::new()?;
        thread.fetch_post(&form.id)
    })
    .await
    .map_err(|_| actix_web::Error::from(ErrorInternalServerError("couldn't fetch post")))?;

    if let Some(mut post) = resp.unwrap() {
        let media = task::spawn_blocking(move || {
            let mut fetched_media = post.media.unwrap_or(vec![]);

            for item in &mut fetched_media {
                item.content = proxy::store(&item.content, store.to_owned())
                    .map_err(|_| ErrorInternalServerError("couldn't store image"))
                    .unwrap(); // fix later

                if let Some(thumbnail) = &item.thumbnail {
                    item.content = proxy::store(&thumbnail, store.to_owned())
                        .map_err(|_| ErrorInternalServerError("couldn't store image"))
                        .unwrap_or(thumbnail.to_owned());
                }
            }

            fetched_media
        })
        .await
        .map_err(|_| ErrorInternalServerError("couldn't spawn thread"))?;

        post.media = Some(media);
        Ok(web::Json(Some(post)))
    } else {
        Err(ErrorNotFound("null"))
    }
}
