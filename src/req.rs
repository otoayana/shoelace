use crate::{
    error::{ProxyError, ShoelaceError},
    proxy, ShoelaceData,
};
use actix_web::web::Data;
use futures::future::join_all;
use serde::Deserialize;
use spools::{Media, Post, Threads, User};

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

/// Common function for storing media structs
async fn media_store(media: &mut Media, store: Data<ShoelaceData>) -> Result<(), ProxyError> {
    media.content = proxy::store(&media.content, store.to_owned()).await?;

    if let Some(thumbnail) = &media.thumbnail {
        media.thumbnail = Some(proxy::store(thumbnail, store.to_owned()).await?);
    }

    Ok(())
}

/// Fetches a user, and proxies its media
pub async fn user(data: UserData, store: Data<ShoelaceData>) -> Result<User, ShoelaceError> {
    // Fetch user
    let thread = Threads::new()?;
    let mut resp = thread.fetch_user(&data.tag).await?;

    // Proxy user's profile picture
    let pfp = proxy::store(resp.pfp.as_str(), store.to_owned()).await?;
    resp.pfp.clone_from(&pfp);

    // Proxy posts
    join_all(resp.posts.iter_mut().map(|sub| {
        // All of these posts should have the same profile picture
        sub.author.pfp.clone_from(&pfp);

        // Objects
        async {
            join_all(sub.media.iter_mut().map(|object| async {
                media_store(object, store.to_owned()).await?;
                Ok::<(), ProxyError>(())
            }))
            .await;
        }
    }))
    .await;

    Ok(resp)
}

/// Fetches a post, and proxies its media
pub async fn post(post: PostData, store: Data<ShoelaceData>) -> Result<Post, ShoelaceError> {
    // Fetch post
    let thread = Threads::new()?;
    let mut resp = thread.fetch_post(&post.id).await?;

    // Proxy author's profile picture
    resp.author.pfp = proxy::store(&resp.author.pfp, store.to_owned()).await?;

    // Oroxy post's media
    join_all(resp.media.iter_mut().map(|object| async {
        media_store(object, store.to_owned()).await?;
        Ok::<(), ProxyError>(())
    }))
    .await;

    // Proxy media in parents
    join_all(resp.parents.iter_mut().map(|sub| async {
        // Profile picture
        sub.author.pfp = proxy::store(&sub.author.pfp, store.to_owned()).await?;

        // Objects
        join_all(sub.media.iter_mut().map(|object| async {
            media_store(object, store.to_owned()).await?;
            Ok::<(), ProxyError>(())
        }))
        .await;

        Ok::<(), ProxyError>(())
    }))
    .await;

    // Proxy media in replies
    join_all(resp.replies.iter_mut().map(|sub| async {
        // Profile picture
        sub.author.pfp = proxy::store(&sub.author.pfp, store.to_owned()).await?;

        // Objects
        join_all(sub.media.iter_mut().map(|object| async {
            media_store(object, store.to_owned()).await?;
            Ok::<(), ProxyError>(())
        }))
        .await;

        Ok::<(), ProxyError>(())
    }))
    .await;

    Ok(resp)
}
