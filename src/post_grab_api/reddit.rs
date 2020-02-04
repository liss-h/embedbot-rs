use super::*;
use serde_json::Value;
use serenity::builder::CreateEmbed;


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

fn fmt_title(p: &RedditPost) -> String {
    let title = limit_len(
        &escape_markdown(&p.title),
        EMBED_TITLE_MAX_LEN - 24 - p.subreddit.len() - p.flair.len()); // -24 for formatting

    if p.flair.is_empty() {
        format!("'{}' - **reddit.com/r/{}**", title, p.subreddit)
    } else {
        format!("'{}' [{}] - **reddit.com/r/{}**", title, p.flair, p.subreddit)
    }
}


fn base_embed<'a>(e: &'a mut CreateEmbed, u: &User, post: &RedditPost) -> &'a mut CreateEmbed {
    e.title(&fmt_title(post))
        .description(&limit_descr_len(&format!("{}\n{}", &post.embed_url, &post.text)))
        .author(|a| a.name(&u.name))
        .url(&post.src)
}


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RedditPostType {
    Text,
    Image,
    Video
}

#[derive(Clone)]
pub struct RedditPost {
    src: String,
    subreddit: String,
    title: String,
    embed_url: String,
    post_type: RedditPostType,
    text: String,
    flair: String,
    nsfw: bool,
}

impl Post for RedditPost {
    fn should_embed(&self) -> bool {
        true
    }

    fn create_embed(&self, u: &User, create_msg: &mut CreateMessage) {
        match self.post_type {
            RedditPostType::Text => create_msg.embed(|e| base_embed(e, u, self)),

            RedditPostType::Image if self.nsfw => create_msg.embed(|e| {
                base_embed(e, u, self)
                    .field("Warning NSFW", "Click to view content", true)
            }),

            RedditPostType::Image => create_msg.embed(|e| {
                base_embed(e, u, self)
                    .image(&self.embed_url)
            }),

            RedditPostType::Video if self.embed_url.ends_with(".gif") => create_msg.content(format!(
                ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{title}\n\n{text}",
                author = &u.name,
                src = &self.src,
                embed_url = &self.embed_url,
                title = fmt_title(self),
                text = limit_descr_len(&self.text),
            )),

            RedditPostType::Video => create_msg.embed(|e| {
                e.title(&fmt_title(self))
                    .description("[click title to watch video]")
                    .author(|a| a.name(&u.name))
                    .url(&self.src)
                    .image(&self.embed_url)
            }),
        };
    }
}



#[derive(Default)]
pub struct RedditAPI;

impl PostGrabAPI for RedditAPI {
    fn is_suitable(&self, url: &str) -> bool {
        url.starts_with("https://www.reddit.com")
    }

    fn get_post(&self, url: &str) -> Result<Box<dyn Post>, Error> {
        let json = wget_json(url, USER_AGENT)?;

        let post_json = json
            .get(0)?
            .get("data")?
            .get("children")?
            .get(0)?
            .get("data")?
            .as_object()?;

        let title = post_json.get("title")?.as_str()?.to_string();

        let (post_type, embed_url) = match post_json.get("secure_media") {
            Some(Value::Object(sm)) if sm.contains_key("reddit_video")
                => (RedditPostType::Video, post_json.get("thumbnail")?.as_str()?.to_string()),

            Some(Value::Object(sm)) if sm.contains_key("oembed")
                => (RedditPostType::Image, sm.get("oembed")?.get("thumbnail_url")?.as_str()?.to_string()),

            _ => {
                let url = post_json.get("url")?.as_str()?.to_string();

                if has_image_extension(&url) {
                    (RedditPostType::Image, url)
                } else {
                    (RedditPostType::Text, url)
                }
            }
        };

        let subreddit = post_json.get("subreddit")?.as_str()?.to_string();
        let text = post_json.get("selftext")?.as_str()?.to_string();

        let flair = match post_json.get("link_flair_text") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };

        let nsfw = post_json.get("over_18")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Box::new(RedditPost {
            src: url.to_string(), // edit remove android sharing stuff
            subreddit,
            title,
            embed_url,
            post_type,
            text,
            flair,
            nsfw,
        }))
    }
}
