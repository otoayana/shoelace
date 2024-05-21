use crate::{
    error::ShoelaceError,
    proxy::{self}, ShoelaceData,
};
use actix_web::web::Data;
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
async fn media_store(media: &mut Media, store: Data<ShoelaceData>) {
    media.content = proxy::store(&media.content, store.to_owned()).await;

    if let Some(thumbnail) = &media.thumbnail {
        media.thumbnail = Some(proxy::store(thumbnail, store.to_owned()).await);
    }
}

/// Fetches a user, and proxies its media
pub async fn user(data: UserData, store: Data<ShoelaceData>) -> Result<User, ShoelaceError> {
    // Fetch user
    let thread = Threads::new()?;
    let mut resp = thread.fetch_user(&data.tag).await?;

    // Proxy user's profile picture
    let pfp = proxy::store(resp.pfp.as_str(), store.to_owned()).await;
    resp.pfp.clone_from(&pfp);

    // Clone user's posts to vector
    let mut posts = resp.posts.clone();

    // Store objects in previous vector
    for item in &mut posts {
        // All of these posts should have the same profile picture
        item.author.pfp.clone_from(&pfp);

        // Objects
        for object in &mut item.media {
            media_store(object, store.to_owned()).await
        }
    }

    // Save proxied media in response
    resp.posts = posts;

    Ok(resp)
}

/// Fetches a post, and proxies its media
pub async fn post(post: PostData, store: Data<ShoelaceData>) -> Result<Post, ShoelaceError> {
    // Fetch post
    let thread = Threads::new()?;
    let mut resp = thread.fetch_post(&post.id).await?;

    // Proxy author's profile picture
    resp.author.pfp = proxy::store(&resp.author.pfp, store.to_owned()).await;

    // Clone post's media to a mutable vector
    let mut media = resp.media;

    // Store objects in previous vector
    for object in &mut media {
        media_store(object, store.to_owned()).await
    }

    // Get post parents
    let mut parents = resp.parents.clone();

    // Store media in parents
    for item in &mut parents {
        // Profile picture
        item.author.pfp = proxy::store(&item.author.pfp, store.to_owned()).await;

        // Objects
        for object in &mut item.media {
            media_store(object, store.to_owned()).await
        }
    }

    // Get post replies
    let mut replies = resp.replies.clone();

    // Store media in replies
    for item in &mut replies {
        // Profile picture
        item.author.pfp = proxy::store(&item.author.pfp, store.to_owned()).await;

        // Objects
        for attachment in &mut item.media {
            media_store(attachment, store.to_owned()).await
        }
    }

    // Save proxied media in response
    resp.media = media;
    resp.parents = parents;
    resp.replies = replies;

    Ok(resp)
}
