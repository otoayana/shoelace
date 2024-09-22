use crate::{req, ShoelaceData};
use askama_axum::IntoResponse;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use axum_xml_up::Xml;
use chrono::{TimeZone, Utc};
use rss::{ChannelBuilder, ImageBuilder, Item, ItemBuilder};
use std::sync::Arc;

pub fn attach(enabled: bool) -> Router<Arc<ShoelaceData>> {
    let mut routed = Router::new();

    if enabled {
        routed = routed.route("/:id", get(user))
    }

    routed
}

// Build an RSS feed for a profile
async fn user(Path(user): Path<String>, State(store): State<Arc<ShoelaceData>>) -> Response {
    let request = req::user(&user, &store).await;

    match request {
        Ok(response) => {
            let items: Vec<Item> = response
                .posts
                .iter()
                .map(|post| {
                    let date = Utc.timestamp_opt(post.date as i64, 0).unwrap();

                    ItemBuilder::default()
                        .title(format!(
                            "Post by {} on {}",
                            response.name,
                            date.format("%Y-%m-%d")
                        ))
                        .link(format!("{}/t/{}", store.config.server.base_url, post.code))
                        .description(post.body.clone())
                        .author(format!("@{}", post.author.username))
                        .pub_date(date.to_rfc2822())
                        .build()
                })
                .collect();

            let pfp = ImageBuilder::default()
                .title(format!("@{}'s profile picture", user.clone()))
                .url(response.pfp.clone())
                .build();

            let channel = ChannelBuilder::default()
                .title(format!("{} (@{})", response.name, user.clone()))
                .link(format!(
                    "{}/@{}",
                    store.config.server.base_url,
                    user.clone()
                ))
                .description(response.bio)
                .image(pfp)
                .items(items)
                .build();

            (StatusCode::OK, Xml(channel.to_string())).into_response()
        }
        Err(error) => error.into_plaintext(),
    }
}
