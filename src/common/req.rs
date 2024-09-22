use crate::{proxy, Error, ShoelaceData};
use futures::future::join_all;
use spools::{Media, Post, Threads, User};

// Common function for storing media structs
async fn media_store(media: &mut Media, store: &ShoelaceData) -> Result<(), proxy::Error> {
    media.content = proxy::store(&media.content, store.clone()).await?;

    media.thumbnail = proxy::store(&media.thumbnail, store.clone()).await?;

    Ok(())
}

// Fetches a user, and proxies its media
#[tracing::instrument(err(Display), skip(user, store), fields(error))]
pub async fn user<'a>(user: &'a str, store: &ShoelaceData) -> Result<User, Error> {
    // Fetch user
    let thread = Threads::new()?;
    let mut resp = thread.fetch_user(&user).await?;

    // Proxy user's profile picture
    let pfp = proxy::store(resp.pfp.as_str(), store.clone()).await?;
    resp.pfp.clone_from(&pfp);

    // Proxy posts
    join_all(resp.posts.iter_mut().map(|sub| {
        // All of these posts should have the same profile picture
        sub.author.pfp.clone_from(&pfp);

        // Objects
        async {
            join_all(sub.media.iter_mut().map(|object| async {
                media_store(object, &store).await?;
                Ok::<(), proxy::Error>(())
            }))
            .await;
        }
    }))
    .await;

    Ok(resp)
}

// Fetches a post, and proxies its media
#[tracing::instrument(err(Display), skip(post, store), fields(error))]
pub async fn post<'a>(post: &'a str, store: &ShoelaceData) -> Result<Post, Error> {
    // Fetch post
    let thread = Threads::new()?;
    let mut resp = thread.fetch_post(&post).await?;
    // Proxy author's profile picture
    resp.author.pfp = proxy::store(&resp.author.pfp, store.clone()).await?;
    // Oroxy post's media
    join_all(resp.media.iter_mut().map(|object| async {
        media_store(object, store).await?;
        Ok::<(), proxy::Error>(())
    }))
    .await;
    // Proxy media in parents
    join_all(resp.parents.iter_mut().map(|sub| async {
        // Profile picture
        sub.author.pfp = proxy::store(&sub.author.pfp, store.to_owned()).await?;
        // Objects
        join_all(sub.media.iter_mut().map(|object| async {
            media_store(object, &store).await?;
            Ok::<(), proxy::Error>(())
        }))
        .await;
        Ok::<(), proxy::Error>(())
    }))
    .await;
    // Proxy media in replies
    join_all(resp.replies.iter_mut().map(|sub| async {
        // Profile picture
        sub.author.pfp = proxy::store(&sub.author.pfp, store.to_owned()).await?;
        // Objects
        join_all(sub.media.iter_mut().map(|object| async {
            media_store(object, &store).await?;
            Ok::<(), proxy::Error>(())
        }))
        .await;
        Ok::<(), proxy::Error>(())
    }))
    .await;

    Ok(resp)
}
