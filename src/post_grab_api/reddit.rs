use super::*;

pub struct RedditAPI;

impl Default for RedditAPI {
    fn default() -> Self {
        Self
    }
}

impl PostGrabAPI for RedditAPI {
    fn get_post(&mut self, url: &str) -> Result<Post, Error> {
        let mut resp = {
            let client = reqwest::Client::new();
            client
                .get(&format!("{}/.json", url))
                .header("User-Agent", "embedbot")
                .send()?
        };

        let json: serde_json::Value = resp.json()?;

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
        let embed_url = post_json.get("url")?.as_str()?.to_string();
        let is_vid = post_json.get("is_video")?.as_bool()?;
        let subreddit = post_json.get("subreddit")?.as_str()?.to_string();

        Ok(Post {
            website: "reddit".to_string(),
            title,
            embed_url,
            post_type: if is_vid {
                PostType::Video
            } else {
                PostType::Image
            },
            origin: format!("reddit.com/r/{}", subreddit),
        })
    }
}
