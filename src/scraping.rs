use crate::{
    proxy::{self, Db},
    utils::query,
};
use actix_web::web::Data;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::vec;
use tokio::task;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub name: String,
    pub body: Option<String>,
    pub media: Option<Vec<Media>>,
    pub likes: u64,
    pub reposts: u64,
    pub parents: Option<Vec<String>>,
    pub replies: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MediaTypes {
    Image,
    Video,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Media {
    pub alt: Option<String>,
    pub content: String,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub pfp: Option<String>,
    pub verified: bool,
    pub bio: Option<String>,
    pub followers: u64,
    pub links: Option<Vec<String>>,
    pub posts: Option<Vec<String>>,
}

/// Fetches information from a user
#[tokio::main]
pub async fn user(tag: &str, store: Option<Data<Db>>) -> Result<Option<User>> {
    let variables: String = format!("\"username\":\"{}\"", tag);
    let resp = task::spawn_blocking(move || query(&variables, "7394812507255098")).await??;

    let parent = resp
        .pointer("/data/xdt_user_by_username")
        .unwrap_or(&Value::Null);

    if parent.is_null() {
        return Ok(None);
    }

    let mut name: Option<String> = None;
    let mut bio: Option<String> = None;

    // these variables need their quotes removed
    let quot = vec!["id", "full_name", "biography"];
    let mut unquot: Vec<String> = vec![];

    for x in quot {
        unquot.push(parent[x].as_str().to_owned().unwrap().to_string())
    }

    let mut pfp: Option<String> = None;
    let pfp_location = parent
        .pointer("/hd_profile_pic_versions")
        .unwrap_or(&Value::Null);
    if pfp_location.is_array() {
        let pfp_versions = pfp_location.as_array().unwrap();
        pfp = Some(
            pfp_versions[pfp_versions.len() - 1]["url"]
                .as_str()
                .to_owned()
                .unwrap()
                .to_string(),
        );
        if let Some(store) = store.clone() {
            pfp = Some(
                task::spawn_blocking(move || proxy::store(&pfp.unwrap_or(String::new()), store))
                    .await??,
            );
        }
    }

    if !unquot[1].is_empty() {
        name = Some(unquot[1].clone())
    }

    if !unquot[2].is_empty() {
        bio = Some(unquot[2].clone())
    }

    // getting additional information through the user ID
    let id_var = format!("\"userID\":\"{}\"", unquot[0]);
    let id_resp = task::spawn_blocking(move || query(&id_var, "25253062544340717")).await??;

    let links_parent = id_resp
        .pointer("/data/user/bio_links")
        .unwrap_or(&Value::Null);
    let mut links: Option<Vec<String>> = None;
    if links_parent.is_array() {
        let mut links_vec: Vec<String> = vec![];
        for x in links_parent.as_array().unwrap() {
            links_vec.push(x["url"].as_str().to_owned().unwrap().to_string())
        }
        links = Some(links_vec);
    }

    // getting user posts
    let mut posts: Option<Vec<String>> = None;
    let post_var = format!("\"userID\":\"{}\"", unquot[0]);
    let post_resp = task::spawn_blocking(move || query(&post_var, "7357407954367176")).await??;
    let edges = post_resp
        .pointer("/data/mediaData/edges")
        .unwrap_or(&Value::Null);
    if edges.is_array() {
        let node_array = edges.as_array().unwrap();
        let mut post_vec: Vec<String> = vec![];
        for y in node_array {
            let thread_items = y.pointer("/node/thread_items").unwrap();
            for x in thread_items.as_array().unwrap() {
                let cur = x.pointer("/post").unwrap();
                let code = cur["code"].as_str().to_owned().unwrap();
                post_vec.push(code.to_string());
            }
        }
        posts = Some(post_vec);
    }

    Ok(Some(User {
        id: unquot[0].parse::<u64>()?,
        name,
        pfp: pfp,
        bio,
        links,
        verified: parent["is_verified"].as_bool().unwrap_or(false),
        followers: parent["follower_count"].as_u64().unwrap_or(0),
        posts,
    }))
}

/// Fetches information from a post
#[tokio::main]
pub async fn post(tag: &str, id: &str, store: Option<Data<proxy::Db>>) -> Result<Option<Post>> {
    // idiomatic way of escaping if the profile doesn't exist
    let variables: String = format!("\"username\":\"{}\"", &tag);
    let resp = task::spawn_blocking(move || query(&variables, "7394812507255098")).await??;

    let user_info = resp
        .pointer("/data/xdt_user_by_username")
        .unwrap_or(&Value::Null);

    if user_info.is_null() {
        return Ok(None);
    };

    // we need to scrape full id to fetch a post through graphql, since there doesn't seem to be an endpoint to fetch the post from the short id
    let values = (tag.to_owned(), id.to_owned());
    let fullid =
        task::spawn_blocking(move || crate::utils::post_id(&values.0, &values.1)).await??;

    let variables = format!("\"postID\":\"{}\"", &fullid);
    let resp = task::spawn_blocking(move || query(&variables, "26262423843344977")).await??;

    let check = resp.pointer("/data/data/edges");

    if check.is_none() {
        return Ok(None);
    }

    // meta wrapping stuff in arrays -.-
    let node_array = check.unwrap_or(&Value::Null).as_array().unwrap();
    let mut parents: Option<Vec<String>> = None;
    let mut replies: Option<Vec<String>> = None;
    let mut post = &Value::Null;
    let mut post_found: bool = false;
    let mut parents_vec: Vec<String> = vec![];
    let mut replies_vec: Vec<String> = vec![];

    for node in node_array {
        let thread_items = node.pointer("/node/thread_items").unwrap_or(&Value::Null);

        if !thread_items.is_array() {
            return Ok(None);
        }

        for item in thread_items.as_array().unwrap() {
            let cur = item.pointer("/post").unwrap();
            let code = cur["code"].as_str().to_owned().unwrap();
            if code == id {
                post = cur;
                post_found = true;
            } else if !post_found {
                parents_vec.push(code.to_string());
                parents = Some(parents_vec.clone());
            } else {
                replies_vec.push(code.to_string());
                replies = Some(replies_vec.clone());
            }
        }
    }

    let body = post
        .pointer("/caption/text")
        .unwrap()
        .as_str()
        .to_owned()
        .unwrap();

    // check media in singular locations
    let video_location = post.pointer("/video_versions").unwrap_or(&Value::Null);
    let image_location = post
        .pointer("/image_versions2/candidates")
        .unwrap_or(&Value::Null);
    let mut media: Option<Vec<Media>> = None;
    let mut media_vec: Vec<Media> = vec![];

    // check media in carousel
    let carousel_location = post.pointer("/carousel_media").unwrap_or(&Value::Null);
    if carousel_location.is_array() {
        let carousel_array = carousel_location.as_array().unwrap();
        for x in carousel_array {
            let item = &x
                .pointer("/image_versions2/candidates")
                .unwrap()
                .as_array()
                .unwrap()[0];
            let media_url = item["url"].as_str().to_owned().unwrap().to_string();
            let mut media_alt: Option<String> = None;
            if !x["accessibility_caption"].is_null() {
                media_alt = Some(
                    x["accessibility_caption"]
                        .as_str()
                        .to_owned()
                        .unwrap()
                        .to_string(),
                );
            }

            let mut media = media_url.clone();

            if let Some(store) = store.clone() {
                media = task::spawn_blocking(move || proxy::store(&media_url, store)).await??;
            }

            media_vec.push(Media {
                alt: media_alt,
                content: media,
                thumbnail: None,
            });
        }
    } else if image_location.is_array() {
        // defines empty and optional values
        let content: String;
        let mut alt: Option<String> = None;
        let mut thumbnail: Option<String> = None;

        let image_array = image_location.as_array().unwrap();
        let image_url = image_array[0]["url"]
            .as_str()
            .to_owned()
            .unwrap()
            .to_string();
        let mut image = image_url.clone();

        // stores image URL in proxy if possible
        if let Some(store) = store.clone() {
            image = task::spawn_blocking(move || proxy::store(&image_url, store)).await??;
        }

        // alt text
        if post["accessibility_caption"].is_string() {
            alt = Some(
                post["accessibility_caption"]
                    .as_str()
                    .to_owned()
                    .unwrap()
                    .to_string(),
            );
        }

        // video
        if video_location.is_array() {
            let video_array = video_location.as_array().unwrap();
            let mut video = video_array[0]["url"]
                .as_str()
                .to_owned()
                .unwrap()
                .to_string();
            if let Some(store) = store.clone() {
                video = task::spawn_blocking(move || proxy::store(&video, store)).await??;
            }
            content = video;
            thumbnail = Some(image);
        } else {
            content = image;
        }

        media_vec.push(Media {
            alt,
            content,
            thumbnail,
        })
    }

    if media_vec.len() != 0 {
        media = Some(media_vec);
    }

    Ok(Some(Post {
        id: fullid,
        name: tag.to_string(),
        body: Some(body.to_string()),
        media,
        likes: post["like_count"].as_u64().unwrap_or(0),
        reposts: post
            .pointer("/text_post_app_info/repost_count")
            .unwrap()
            .as_u64()
            .unwrap_or(0),
        parents,
        replies,
    }))
}
