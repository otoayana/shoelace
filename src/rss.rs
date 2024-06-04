use crate::{req, Error, ShoelaceData};
use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data},
    HttpResponse, Result,
};
use rss::{ChannelBuilder, ImageBuilder, Item, ItemBuilder};

// Build an RSS feed for a profile
#[get("/{user}")]
pub(crate) async fn user(
    user: web::Path<String>,
    store: Data<ShoelaceData>,
) -> Result<HttpResponse, Error> {
    // Fetch user
    let request = req::user(user.clone(), store.to_owned()).await?;

    // Serialize posts
    let items: Vec<Item> = request
        .posts
        .iter()
        .map(|post| {
            ItemBuilder::default()
                .title(post.body.clone())
                .author(format!("@{}", post.author.username))
                .description(post.body.clone())
                .link(format!("{}/t/{}", store.base_url, post.code))
                .build()
        })
        .collect();

    // Include profile picture
    let pfp = ImageBuilder::default()
        .title(format!("@{}'s profile picture", user.clone()))
        .url(request.pfp.clone())
        .build();

    // Build channel
    let channel = ChannelBuilder::default()
        .title(format!("{} (@{})", request.name, user.clone()))
        .description(request.bio)
        .link(format!("{}/@{}", store.base_url, user.clone()))
        .items(items)
        .image(pfp)
        .build();

    // Return feed
    Ok(HttpResponse::Ok()
        .content_type(ContentType::xml())
        .body(channel.to_string()))
}
