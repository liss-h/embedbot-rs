use super::*;
use serde_json::Value;


fn has_image_extension(s: &str) -> bool {
    const EXTENSIONS: [&'static str; 11] = [
        ".jpg",
        ".png",
        ".gif",
        ".tif",
        ".bmp",
        ".dib",
        ".jpeg",
        ".jpe",
        ".jfif",
        ".tiff",
        ".heic",
    ];

    EXTENSIONS
        .iter()
        .any(|x| s.ends_with(x))
}


#[derive(Default)]
pub struct RedditAPI;

impl PostGrabAPI for RedditAPI {
    fn is_suitable(&self, url: &str) -> bool {
        url.starts_with("https://www.reddit.com")
    }

    fn get_post(&self, url: &str) -> Result<Post, Error> {
        let json = wget_json(url, USER_AGENT)?;

        let post_json = json
            .as_array()?
            .get(0)?
            .as_object()?
            .get("data")?
            .as_object()?
            .get("children")?
            .as_array()?
            .get(0)?
            .as_object()?
            .get("data")?
            .as_object()?;

        let title = post_json.get("title")?.as_str()?.to_string();

        // let is_vid_tag = post_json.get("is_video")?.as_bool()?;

        let (post_type, embed_url) = match post_json.get("secure_media") {
            Some(Value::Object(sm)) if sm.contains_key("reddit_video")
                => (PostType::Video, post_json.get("thumbnail")?.as_str()?.to_string()),

            Some(Value::Object(sm)) if sm.contains_key("oembed")
                => (PostType::Image, sm.get("oembed")?.as_object()?.get("thumbnail_url")?.as_str()?.to_string()),

            _ => {
                let url = post_json.get("url")?.as_str()?.to_string();

                if has_image_extension(&url) {
                    (PostType::Image, url)
                } else {
                    (PostType::Text, url)
                }
            }
        };


        /*let embed_url = if is_vid_tag {
            // use thumbnail as embedurl
            post_json.get("thumbnail")?.as_str()?.to_string()
        } else {
            post_json.get("url")?.as_str()?.to_string()
        };

        let post_type =
            if embed_url.ends_with(".gif") {
                PostType::Video
            } else {
                match post_json.get("secure_media") {
                    None | Some(serde_json::Value::Null) => match post_json.get("post_hint") {
                        Some(serde_json::Value::String(s)) if s.contains("video") => PostType::Video,
                        Some(serde_json::Value::String(s)) if s == "image" => PostType::Image,
                        None if has_image_extension(&embed_url) => PostType::Image,
                        _ => PostType::Text,
                    },
                    
                    Some(_) => PostType::Video,
                }
            };*/

        let subreddit = post_json.get("subreddit")?.as_str()?.to_string();
        let text = post_json.get("selftext")?.as_str()?.to_string();

        let flair = match post_json.get("link_flair_text") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };

        let nsfw = post_json.get("over_18")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Post {
            website: "reddit".to_string(),
            origin: format!("reddit.com/r/{}", subreddit),
            title,
            embed_url,
            post_type,
            text,
            flair,
            nsfw,
        })
    }
}
