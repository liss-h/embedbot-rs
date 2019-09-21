use super::*;

#[derive(Default)]
pub struct RedditAPI;

impl PostGrabAPI for RedditAPI {
    fn get_post(&mut self, url: &str) -> Result<Post, Error> {
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
        let is_vid = post_json.get("is_video")?.as_bool()?;

        let embed_url = if is_vid {
            // use thumbnail as embedurl
            post_json.get("thumbnail")?.as_str()?.to_string()
        } else {
            let tmp = post_json.get("url")?.as_str()?.to_string();

            if tmp.contains("imgur.com/") {
                if tmp.ends_with(".gifv") {
                    tmp[0..(tmp.len() - 1)].to_string()
                } else {
                    let mut imgur = imgur::ImgurAPI::default();

                    match imgur.get_post(&tmp) {
                        Ok(post) => post.embed_url,
                        Err(_) => tmp
                    }
                }
            } else {
                tmp
            }
        };

        let subreddit = post_json.get("subreddit")?.as_str()?.to_string();
        let text = post_json.get("selftext")?.as_str()?.to_string();

        Ok(Post {
            website: "reddit".to_string(),
            title,
            embed_url,
            post_type: if is_vid {
                PostType::Video
            } else if text.is_empty() {
                PostType::Image
            } else {
                PostType::Text
            },
            origin: format!("reddit.com/r/{}", subreddit),
            text,
        })
    }
}
