use serde_json::Value;
use serenity::builder::CreateEmbed;

use super::*;
use crate::embed_bot::is_url;


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
    let flair = (!p.common.flair.is_empty()).then_some(format!("[{}] ", p.common.flair)).unwrap_or_default();

    match &p.common.subreddit {
        RedditPostOrigin::Crossposted { from, to } => {
            let title = limit_len(
                &escape_markdown(&p.common.title),
                EMBED_TITLE_MAX_LEN - 34 - 18 - to.len() - flair.len() - from.len()); // -34-18 for formatting

            format!("'{title}' {flair}- **reddit.com/r/{subreddit}\n[XPosted from r/{from}]**",
                    title = title,
                    flair = flair,
                    subreddit = to,
                    from = from
            )
        },
        RedditPostOrigin::JustSubreddit(subreddit) => {
            let title = limit_len(
                &escape_markdown(&p.common.title),
                EMBED_TITLE_MAX_LEN - 34 - subreddit.len() - flair.len()); // -34 for formatting

            format!("'{title}' {flair}- **reddit.com/r/{subreddit}**",
                    title = title,
                    flair = flair,
                    subreddit = subreddit,
            )
        }
    }


}


fn base_embed<'a>(e: &'a mut CreateEmbed, u: &User, post: &RedditPost) -> &'a mut CreateEmbed {
    e.title(&fmt_title(&post))
        .description(&limit_descr_len(&format!("{}", &post.common.text)))
        .author(|a| a.name(&u.name))
        .url(&post.common.src)
}

fn strip_url(url: &str) -> &str {
    let question_mark_pos = url.chars().position(|c| c == '?');
    match question_mark_pos {
        Some(pos) => &url[0..pos],
        None => url
    }
}

fn sanitize_url(url: &str) -> String {
    url.replace("&amp;", "&")
}


#[derive(Clone, Debug)]
pub enum RedditPostOrigin {
    JustSubreddit(String),
    Crossposted {
        from: String,
        to: String
    }
}

#[derive(Clone, Debug)]
pub struct RedditPostCommonData {
    src: String,
    subreddit: RedditPostOrigin,
    title: String,
    text: String,
    flair: String,
    nsfw: bool
}

#[derive(Clone, Debug)]
pub enum RedditPostSpecializedData {
    Text,
    Gallery {
        img_urls: Vec<String>,
    },
    Image {
        img_url: String
    },
    Video {
        video_url: String
    }
}

#[derive(Clone, Debug)]
pub struct RedditPost {
    common: RedditPostCommonData,
    specialized: RedditPostSpecializedData
}


impl Post for RedditPost {
    fn should_embed(&self) -> bool {
        true
    }

    fn create_embed(&self, u: &User, create_msg: &mut CreateMessage) {
        if self.common.nsfw {
            create_msg.embed(|e| {
                e.title(&fmt_title(self))
                    .description("Warning NSFW: Click to view content")
                    .author(|a| a.name(&u.name))
                    .url(&self.common.src)
            })
        } else {
            match &self.specialized {
                RedditPostSpecializedData::Text => create_msg.embed(|e| base_embed(e, u, self)),
                RedditPostSpecializedData::Image { img_url } => {
                    create_msg.embed(|e| base_embed(e, u, self)
                        .image(&img_url))
                },
                RedditPostSpecializedData::Gallery { img_urls } => {
                    let urls = img_urls.iter().fold(String::new(), |acc, url| acc + url + "\n");

                    create_msg.content(format!(
                        ">>> **{author}**\nSource: <{src}>\nEmbedURLs:\n{embed_url}\n{title}\n\n{text}",
                        author = &u.name,
                        src = &self.common.src,
                        embed_url = &urls,
                        title = fmt_title(self),
                        text = limit_descr_len(&self.common.text),
                    ))
                },
                RedditPostSpecializedData::Video { video_url } => create_msg.content(format!(
                    ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{title}\n\n{text}",
                    author = &u.name,
                    src = &self.common.src,
                    embed_url = video_url,
                    title = fmt_title(self),
                    text = limit_descr_len(&self.common.text),
                ))
            }
        };
    }
}



#[derive(Default)]
pub struct RedditAPI;

impl PostScraper for RedditAPI {
    fn is_suitable(&self, url: &str) -> bool {
        url.starts_with("https://www.reddit.com")
    }

    fn get_post(&self, url: &str) -> Result<Box<dyn Post>, Error> {
        let url = url.rfind("/?")
            .map(|idx| &url[..idx])
            .unwrap_or(url);

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

        let common_data = RedditPostCommonData {
            src: url.to_string(),
            subreddit: if is_xpost {
                RedditPostOrigin::Crossposted { from: original_subreddit, to: subreddit }
            } else {
                RedditPostOrigin::JustSubreddit(subreddit)
            },
            title,
            text,
            flair,
            nsfw
        };


        let specialized_data = match post_json.get("secure_media") {
            Some(Value::Object(sm)) if sm.contains_key("reddit_video")
                => RedditPostSpecializedData::Video {
                    video_url: sm.get("reddit_video")?
                        .get("fallback_url")?
                        .as_str()
                        .map(strip_url)?
                        .to_string()
                },

            Some(Value::Object(sm)) if sm.contains_key("oembed")
                => RedditPostSpecializedData::Image {
                    img_url: sm.get("oembed")?
                        .get("thumbnail_url")?
                        .as_str()?
                        .to_string()
                },

            _ => match post_json.get("media_metadata") {
                Some(Value::Object(meta)) => {
                    RedditPostSpecializedData::Gallery {
                        img_urls: meta.iter()
                            .map(|(_key, imgmeta)| {
                                imgmeta.get("s")
                                    .and_then(Value::as_object)
                                    .and_then(|inner| inner.get("u"))
                                    .and_then(Value::as_str)
                                    .map(sanitize_url)
                            }).collect::<Option<Vec<_>>>()?
                    }
                },
                _ => {
                    let url = post_json.get("url")?.as_str()?.to_string();
                    if has_image_extension(&url) {
                        RedditPostSpecializedData::Image {
                            img_url: url
                        }
                    } else {
                        RedditPostSpecializedData::Text
                    }
                }
            }
        };

        // embed_url can be "default" when the original post (referenced by crosspost) is deleted
        let alt_embed_url = top_level_post.get("thumbnail")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .map(|s| if is_url(&s) { s } else { String::new() })
            .unwrap_or_else(Default::default);


        Ok(Box::new(RedditPost{ common: common_data, specialized: match specialized_data {
            RedditPostSpecializedData::Image { img_url } if !is_url(&img_url)
                => RedditPostSpecializedData::Image { img_url: alt_embed_url },

            other => other
        }}))
    }
}
