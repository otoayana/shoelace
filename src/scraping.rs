use crate::{
    proxy::{self, KeyStore},
    utils::query,
};
use actix_web::web::Data;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::vec;
use tokio::task;

#[derive(Debug, Serialize, Deserialize)]
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
pub enum MediaKind {
    Image,
    Video,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Media {
    pub kind: MediaKind,
    pub alt: Option<String>,
    pub content: String,
    pub thumbnail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
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
pub async fn user(tag: &str, store: Option<Data<KeyStore>>) -> Result<Option<User>> {
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
pub async fn post(id: &str, store: Option<Data<proxy::KeyStore>>) -> Result<Option<Post>> {
    // Since there's no endpoint for getting full IDs out of short ones, fetch it from post URL
    let inner_id = id.to_owned();
    let id_req = task::spawn_blocking(move || crate::utils::post_id(&inner_id)).await??;

    if id_req.is_none() {
        return Ok(None);
    }

    let fullid = id_req.unwrap_or(String::new());
    // Now we can fetch the actual post
    let variables = format!("\"postID\":\"{}\"", &fullid);
    let resp = task::spawn_blocking(move || query(&variables, "26262423843344977")).await??;

    let check = resp.pointer("/data/data/edges");

    if check.is_none() {
        return Ok(None);
    }

    // Define values for parents and replies
    let mut parents: Option<Vec<String>> = None;
    let mut replies: Option<Vec<String>> = None;

    let mut parents_vec: Vec<String> = vec![];
    let mut replies_vec: Vec<String> = vec![];

    let mut post = &Value::Null;
    let mut post_found: bool = false;

    // Meta wrapping stuff in arrays -.-
    let node_array = check.unwrap_or(&Value::Null).as_array().unwrap();

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

    // Get the post's author
    let tag = post
        .pointer("/user/username")
        .unwrap()
        .as_str()
        .to_owned()
        .unwrap();

    // Get the post's body
    let body = post
        .pointer("/caption/text")
        .unwrap()
        .as_str()
        .to_owned()
        .unwrap();

    // Locations for singular media
    let video_location = post.pointer("/video_versions").unwrap_or(&Value::Null);
    let image_location = post
        .pointer("/image_versions2/candidates")
        .unwrap_or(&Value::Null);

    // Locations for carousel media
    let carousel_location = post.pointer("/carousel_media").unwrap_or(&Value::Null);

    // Define media variables
    let mut media: Option<Vec<Media>> = None;
    let mut media_vec: Vec<Media> = vec![];

    // Check where media could be
    if carousel_location.is_array() {
        let carousel_array = carousel_location.as_array().unwrap();
        for node in carousel_array {
            // Initial values
            let mut kind = MediaKind::Image;
            let content: String;
            let mut alt: Option<String> = None;
            let mut thumbnail: Option<String> = None;

            // Image
            let node_image_location = &node
                .pointer("/image_versions2/candidates")
                .unwrap()
                .as_array()
                .unwrap()[0];
            let node_video_location = node.pointer("/video_versions").unwrap_or(&Value::Null);

            // CDN URL
            let image_url = node_image_location["url"]
                .as_str()
                .to_owned()
                .unwrap()
                .to_string();

            // Alt text
            if !node["accessibility_caption"].is_null() {
                alt = Some(
                    node["accessibility_caption"]
                        .as_str()
                        .to_owned()
                        .unwrap()
                        .to_string(),
                );
            }

            let mut image = image_url.clone();

            // Store URL in keystore if applicable
            if let Some(store) = store.clone() {
                image = task::spawn_blocking(move || proxy::store(&image_url, store)).await??;
            }

            // Video
            if node_video_location.is_array() {
                let video_array = node_video_location.as_array().unwrap();

                let mut video = video_array[0]["url"]
                    .as_str()
                    .to_owned()
                    .unwrap()
                    .to_string();

                // Store URL in keystore if applicable
                if let Some(store) = store.clone() {
                    video = task::spawn_blocking(move || proxy::store(&video, store)).await??;
                }

                kind = MediaKind::Video;
                content = video;
                thumbnail = Some(image);
            } else {
                content = image;
            }

            media_vec.push(Media {
                kind,
                alt,
                content,
                thumbnail,
            });
        }
    } else if image_location.is_array() && image_location.as_array().unwrap_or(&vec![]).len() != 0 {
        // Initial values
        let mut kind = MediaKind::Image;
        let content: String;
        let mut alt: Option<String> = None;
        let mut thumbnail: Option<String> = None;

        // Gets the first image in URL, since it's in the highest quality
        let image_array = image_location.as_array().unwrap();

        let image_url = image_array[0]["url"]
            .as_str()
            .to_owned()
            .unwrap()
            .to_string();

        // Alt text
        if post["accessibility_caption"].is_string() {
            alt = Some(
                post["accessibility_caption"]
                    .as_str()
                    .to_owned()
                    .unwrap()
                    .to_string(),
            );
        }

        let mut image = image_url.clone();

        // Store URL in keystore if applicable
        if let Some(store) = store.clone() {
            image = task::spawn_blocking(move || proxy::store(&image_url, store)).await??;
        }

        // Video
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

            kind = MediaKind::Video;
            content = video;
            thumbnail = Some(image);
        } else {
            content = image;
        }

        media_vec.push(Media {
            kind,
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
