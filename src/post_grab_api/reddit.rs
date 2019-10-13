use super::*;

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

        let is_vid_tag = post_json.get("is_video")?.as_bool()?;

        let embed_url = if is_vid_tag {
            // use thumbnail as embedurl
            post_json.get("thumbnail")?.as_str()?.to_string()
        } else {
            let tmp = post_json.get("url")?.as_str()?.to_string();
            let imgur = imgur::ImgurAPI::default();

            if imgur.is_suitable(&tmp) {

                if tmp.ends_with(".gifv") {
                    tmp
                }
                else {
                    match imgur.get_post(&tmp) {
                        Ok(post) => post.embed_url,
                        Err(_) => tmp
                    }
                }
            } else {
                tmp
            }
        };

        let post_type =
            if embed_url.ends_with(".gif") {
                PostType::Video
            } else {
                match post_json.get("post_hint") {
                    Some(serde_json::Value::String(s)) if s == "hosted:video" || s == "rich:video" => PostType::Video,
                    Some(serde_json::Value::String(s)) if s == "image" => PostType::Image,
                    None => PostType::Text,
                    _ => PostType::Image,
                }
            };

        let subreddit = post_json.get("subreddit")?.as_str()?.to_string();
        let text = post_json.get("selftext")?.as_str()?.to_string();

        let flair = post_json.get("link_flair_text")?.as_str()?.to_string();

        Ok(Post {
            website: "reddit".to_string(),
            origin: format!("reddit.com/r/{}", subreddit),
            title,
            embed_url,
            post_type,
            text,
            flair,
        })
    }
}
