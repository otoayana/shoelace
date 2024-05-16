use crate::proxy::{self, KeyStore};
use actix_web::web::Data;
use anyhow::Result;
use serde::Deserialize;
use spools::{Post, Threads, User};
use tokio::task;

/// Required values for User endpoint
#[derive(Deserialize)]
pub struct UserData {
    pub tag: String,
}

/// Required values for Post endpoint
#[derive(Deserialize)]
pub struct PostData {
    pub id: String,
}

pub async fn user(data: UserData, store: Data<KeyStore>) -> Result<Option<User>> {
    let thread = Threads::new()?;

    let resp = task::spawn(async move { thread.fetch_user(&data.tag).await }).await?;

    if let Some(mut user) = resp.unwrap() {
        let image = task::spawn_blocking(move || {
            if let Some(pfp) = user.pfp {
                proxy::store(pfp.as_str(), store)
            } else {
                Ok(String::new())
            }
        })
        .await??;

        user.pfp = Some(image);

        Ok(Some(user))
    } else {
        Ok(None)
    }
}

pub async fn post(post: PostData, store: Data<KeyStore>) -> Result<Option<Post>> {
    let thread = Threads::new()?;

    let resp = task::spawn(async move { thread.fetch_post(&post.id).await }).await?;

    if let Some(mut post) = resp.unwrap() {
        let media = task::spawn_blocking(move || {
            let mut fetched_media = post.media.unwrap_or(vec![]);

            for item in &mut fetched_media {
                item.content = proxy::store(&item.content, store.to_owned()).unwrap(); // fix later

                if let Some(thumbnail) = &item.thumbnail {
                    item.content =
                        proxy::store(&thumbnail, store.to_owned()).unwrap_or(thumbnail.to_owned());
                }
            }

            fetched_media
        })
        .await?;

        post.media = Some(media);

        Ok(Some(post))
    } else {
        Ok(None)
    }
}
