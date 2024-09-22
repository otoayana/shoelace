use crate::{proxy, Error, ShoelaceData};
use futures::future::join_all;
use spools::{Media, Post, Threads, User};

/// Common function for storing media structs
async fn media_store(media: &mut Media, store: &ShoelaceData) -> Result<(), proxy::Error> {
    media.content = proxy::store(&media.content, store.clone()).await?;

    media.thumbnail = proxy::store(&media.thumbnail, store.clone()).await?;

    Ok(())
}

/// Fetches a user, and proxies its media
#[tracing::instrument(err(Display), skip(user, store), fields(error))]
pub async fn user<'a>(user: &'a str, store: &ShoelaceData) -> Result<User, Error> {
    let thread = Threads::new()?;
    let mut resp = thread.fetch_user(user).await?;

    let pfp = proxy::store(resp.pfp.as_str(), store.clone()).await?;
    resp.pfp.clone_from(&pfp);

    join_all(resp.posts.iter_mut().map(|sub| {
        // All of these posts should have the same profile picture
        sub.author.pfp.clone_from(&pfp);

        async {
            join_all(sub.media.iter_mut().map(|object| async {
                media_store(object, store).await?;
                Ok::<(), proxy::Error>(())
            }))
            .await;
        }
    }))
    .await;

    Ok(resp)
}

/// Fetches a post, and proxies its media
#[tracing::instrument(err(Display), skip(post, store), fields(error))]
pub async fn post<'a>(post: &'a str, store: &ShoelaceData) -> Result<Post, Error> {
    let thread = Threads::new()?;
    let mut resp = thread.fetch_post(post).await?;
    resp.author.pfp = proxy::store(&resp.author.pfp, store.clone()).await?;

    join_all(resp.media.iter_mut().map(|object| async {
        media_store(object, store).await?;
        Ok::<(), proxy::Error>(())
    }))
    .await;

    join_all(resp.parents.iter_mut().map(|sub| async {
        sub.author.pfp = proxy::store(&sub.author.pfp, store.to_owned()).await?;
        join_all(sub.media.iter_mut().map(|object| async {
            media_store(object, store).await?;
            Ok::<(), proxy::Error>(())
        }))
        .await;
        Ok::<(), proxy::Error>(())
    }))
    .await;

    join_all(resp.replies.iter_mut().map(|sub| async {
        sub.author.pfp = proxy::store(&sub.author.pfp, store.to_owned()).await?;
        join_all(sub.media.iter_mut().map(|object| async {
            media_store(object, store).await?;
            Ok::<(), proxy::Error>(())
        }))
        .await;
        Ok::<(), proxy::Error>(())
    }))
    .await;

    Ok(resp)
}
