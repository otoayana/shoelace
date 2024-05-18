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

pub async fn user(data: UserData, store: Data<KeyStore>) -> Result<User> {
    let thread = Threads::new()?;

    let mut resp = task::spawn(async move { thread.fetch_user(&data.tag).await }).await??;
    let image = proxy::store(resp.pfp.as_str(), store.to_owned()).await?;
    resp.pfp = image.clone();

    let mut posts = resp.posts.clone();

    for item in &mut posts {
        item.author.pfp = image.clone();
        for attachment in &mut item.media {
            attachment.content = proxy::store(&attachment.content, store.to_owned()).await?;

            if let Some(image) = &attachment.thumbnail {
                attachment.thumbnail = Some(
                    proxy::store(&image, store.to_owned())
                        .await
                        .unwrap_or(image.to_owned()),
                );
            }
        }
    }

    resp.posts = posts;

    Ok(resp)
}

pub async fn post(post: PostData, store: Data<KeyStore>) -> Result<Post> {
    let thread = Threads::new()?;

    let mut resp = task::spawn(async move { thread.fetch_post(&post.id).await }).await??;
    resp.author.pfp = proxy::store(&resp.author.pfp, store.to_owned()).await?;

    let mut fetched_media = resp.media;

    for item in &mut fetched_media {
        item.content = proxy::store(&item.content, store.to_owned()).await?;

        if let Some(image) = &item.thumbnail {
            item.thumbnail = Some(
                proxy::store(&image, store.to_owned())
                    .await
                    .unwrap_or(image.to_owned()),
            );
        }
    }

    let mut parents = resp.parents.clone();

    for item in &mut parents {
        item.author.pfp = proxy::store(&item.author.pfp, store.to_owned()).await?;

        for attachment in &mut item.media {
            attachment.content = proxy::store(&attachment.content, store.to_owned()).await?;

            if let Some(image) = &attachment.thumbnail {
                attachment.thumbnail = Some(
                    proxy::store(&image, store.to_owned())
                        .await
                        .unwrap_or(image.to_owned()),
                );
            }
        }
    }

    let mut replies = resp.replies.clone();

    for item in &mut replies {
        item.author.pfp = proxy::store(&item.author.pfp, store.to_owned()).await?;

        for attachment in &mut item.media {
            attachment.content = proxy::store(&attachment.content, store.to_owned()).await?;

            if let Some(image) = &attachment.thumbnail {
                attachment.thumbnail = Some(
                    proxy::store(&image, store.to_owned())
                        .await
                        .unwrap_or(image.to_owned()),
                );
            }
        }
    }

    resp.media = fetched_media;
    resp.parents = parents;
    resp.replies = replies;

    Ok(resp)
}
