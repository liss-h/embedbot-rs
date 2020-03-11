use super::*;
use serde_json::Value;
use serenity::builder::CreateEmbed;
use crate::is_url;


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
    let flair = (!p.flair.is_empty()).then_some(format!("[{}] ", p.flair)).unwrap_or_default();
    let xpost = p.is_xpost.then_some(format!("\n[XPosted from r/{}]", p.original_subreddit)).unwrap_or_default();

    let title = limit_len(
        &escape_markdown(&p.title),
        EMBED_TITLE_MAX_LEN - 34 - p.subreddit.len() - flair.len() - xpost.len()); // -34 for formatting

    format!("'{title}' {flair}- **reddit.com/r/{subreddit}{xpost_marker}**",
        title = title,
        flair = flair,
        subreddit = p.subreddit,
        xpost_marker = xpost
    )
}


fn base_embed<'a>(e: &'a mut CreateEmbed, u: &User, post: &RedditPost) -> &'a mut CreateEmbed {
    e.title(&fmt_title(&post))
        .description(&limit_descr_len(&format!("{}", &post.text)))
        .author(|a| a.name(&u.name))
        .url(&post.src)
}


#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum RedditPostType {
    Text,
    Image,
    Video
}

#[derive(Clone, Debug)]
pub struct RedditPost {
    src: String,
    subreddit: String,
    original_subreddit: String,
    title: String,
    embed_url: String,
    post_type: RedditPostType,
    text: String,
    flair: String,
    nsfw: bool,
    is_xpost: bool,
}

impl Post for RedditPost {
    fn should_embed(&self) -> bool {
        true
    }

    fn create_embed(&self, u: &User, create_msg: &mut CreateMessage) {
        if self.nsfw {
            create_msg.embed(|e| {
                e.title(&fmt_title(self))
                    .description("Warning NSFW: Click to view content")
                    .author(|a| a.name(&u.name))
                    .url(&self.src)
            })
        } else {
            match self.post_type {
                RedditPostType::Text => create_msg.embed(|e| base_embed(e, u, self)),

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
                        .description("Click to watch video")
                        .author(|a| a.name(&u.name))
                        .url(&self.src)
                        .image(&self.embed_url)
                }),
            }
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
        let json = wget_json(&format!("{}/.json", url), USER_AGENT)?;

        let top_level_post = json
            .get(0)?
            .get("data")?
            .get("children")?
            .get(0)?
            .get("data")?
            .as_object()?;

        let title = top_level_post.get("title")?
            .as_str()?
            .to_string();

        let subreddit = top_level_post.get("subreddit")?
            .as_str()?
            .to_string();

        // post_json is either top_level_post or the original post (in case of crosspost)
        let (is_xpost, post_json)
            = top_level_post.get("crosspost_parent_list")
                .and_then(|arr| arr.get(0))
                .and_then(Value::as_object)
                .map(|parent| (true, parent))
                .unwrap_or((false, top_level_post));

        let (post_type, embed_url) = match post_json.get("secure_media") {
            Some(Value::Object(sm)) if sm.contains_key("reddit_video")
                => (
                    RedditPostType::Video,
                    post_json.get("thumbnail")?
                        .as_str()?
                        .to_string()
                ),

            Some(Value::Object(sm)) if sm.contains_key("oembed")
                => (
                    RedditPostType::Image,
                     sm.get("oembed")?
                         .get("thumbnail_url")?
                         .as_str()?
                         .to_string()
                ),

            _ => {
                let url = post_json.get("url")?.as_str()?.to_string();

                if has_image_extension(&url) { (RedditPostType::Image, url) }
                else { (RedditPostType::Text, url) }
            }
        };

        let original_subreddit = post_json.get("subreddit")?
            .as_str()?
            .to_string();

        let text = post_json.get("selftext")?
            .as_str()?
            .to_string();

        let flair = post_json.get("link_flair_text")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_default();

        let nsfw = post_json.get("over_18")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // embed_url can be "default" when the original post (referenced by crosspost) is deleted
        let alt_embed_url = top_level_post.get("thumbnail")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .map(|s| if is_url(&s) { s } else { String::new() })
            .unwrap_or_else(Default::default);

        Ok(Box::new(RedditPost {
            src: url.to_string(), // TODO: remove android sharing stuff
            subreddit,
            original_subreddit,
            title,
            embed_url: if is_url(&embed_url) { embed_url } else { alt_embed_url },
            post_type,
            text,
            flair,
            nsfw,
            is_xpost,
        }))
    }
}
