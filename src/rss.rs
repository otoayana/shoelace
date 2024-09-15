use crate::{req, ShoelaceData};
use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data},
    HttpResponse, Responder, Result,
};
use chrono::{TimeZone, Utc};
use rss::{ChannelBuilder, ImageBuilder, Item, ItemBuilder};

// Build an RSS feed for a profile
// #[get("/{user}")]
// pub(crate) async fn user(
//     user: web::Path<String>,
//     store: Data<ShoelaceData>,
// ) -> Result<impl Responder> {
//     let request = req::user(user.clone(), store.to_owned()).await;

//     match request {
//         Ok(response) => {
//             let items: Vec<Item> = response
//                 .posts
//                 .iter()
//                 .map(|post| {
//                     let date = Utc.timestamp_opt(post.date as i64, 0).unwrap();

//                     ItemBuilder::default()
//                         .title(format!(
//                             "Post by {} on {}",
//                             response.name,
//                             date.format("%Y-%m-%d")
//                         ))
//                         .link(format!("{}/t/{}", store.base_url, post.code))
//                         .description(post.body.clone())
//                         .author(format!("@{}", post.author.username))
//                         .pub_date(date.to_rfc2822())
//                         .build()
//                 })
//                 .collect();

//             let pfp = ImageBuilder::default()
//                 .title(format!("@{}'s profile picture", user.clone()))
//                 .url(response.pfp.clone())
//                 .build();

//             let channel = ChannelBuilder::default()
//                 .title(format!("{} (@{})", response.name, user.clone()))
//                 .link(format!("{}/@{}", store.base_url, user.clone()))
//                 .description(response.bio)
//                 .image(pfp)
//                 .items(items)
//                 .build();

//             Ok(HttpResponse::Ok()
//                 .content_type(ContentType::xml())
//                 .body(channel.to_string()))
//         }
//         Err(error) => Err(error.into_plaintext()),
//     }
// }
