use serde_json::Value;
use serenity::async_trait;
use serenity::builder::CreateEmbed;

use super::*;
use std::convert::TryInto;

fn fmt_title(p: &RedditPostCommonData) -> String {
    let flair = (!p.flair.is_empty())
        .then(|| format!("[{}] ", p.flair))
        .unwrap_or_default();

    match &p.subreddit {
        RedditPostOrigin::Crossposted { from, to } => {
            let em = escape_markdown(&p.title);
            let title = limit_len(
                &em,
                EMBED_TITLE_MAX_LEN - 34 - 18 - to.len() - flair.len() - from.len(),
            ); // -34-18 for formatting

            format!(
                "'{title}' {flair}- **reddit.com/r/{subreddit}\n[XPosted from r/{from}]**",
                title = title,
                flair = flair,
                subreddit = to,
                from = from
            )
        }
        RedditPostOrigin::JustSubreddit(subreddit) => {
            let em = escape_markdown(&p.title);
            let title = limit_len(
                &em,
                EMBED_TITLE_MAX_LEN - 34 - subreddit.len() - flair.len(),
            ); // -34 for formatting

            format!(
                "'{title}' {flair}- **reddit.com/r/{subreddit}**",
                title = title,
                flair = flair,
                subreddit = subreddit,
            )
        }
    }
}

fn base_embed<'a>(
    e: &'a mut CreateEmbed,
    u: &User,
    comment: Option<&str>,
    post: &RedditPost,
) -> &'a mut CreateEmbed {
    e.title(&fmt_title(&post.common))
        .description(limit_descr_len(&escape_markdown(&post.common.text)))
        .author(|a| a.name(&u.name))
        .url(&post.common.src);

    if let Some(comment) = comment {
        include_author_comment(e, u, comment);
    }

    if let Some(comment) = &post.common.comment {
        include_comment(e, comment);
    }

    e
}

fn include_comment<'a>(e: &'a mut CreateEmbed, comment: &RedditComment) -> &'a mut CreateEmbed {
    let name = format!("Comment by Reddit User '{author}'", author = comment.author);
    e.field(name, escape_markdown(&comment.body), true)
}

fn include_author_comment<'a>(
    e: &'a mut CreateEmbed,
    u: &User,
    comment: &str,
) -> &'a mut CreateEmbed {
    let title = format!("Comment by {author}", author = u.name);
    e.field(title, comment, false)
}

fn unescape_url(url: &str) -> String {
    url.replace("&amp;", "&")
}

fn unescape_html(html: &str) -> String {
    html.replace("&amp;", "&")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&quot;", "\"")
}

#[derive(Clone, Debug)]
pub enum RedditPostOrigin {
    JustSubreddit(String),
    Crossposted { from: String, to: String },
}

#[derive(Clone, Debug)]
pub struct RedditComment {
    author: String,
    body: String,
}

#[derive(Clone, Debug)]
pub enum RedditPostShowMode {
    Default,
    Nsfw,
    Spoiler,
}

#[derive(Clone, Debug)]
pub struct RedditPostCommonData {
    src: Url,
    subreddit: RedditPostOrigin,
    title: String,
    text: String,
    flair: String,
    show_mode: RedditPostShowMode,
    comment: Option<RedditComment>,
}

#[derive(Clone, Debug)]
pub enum RedditPostSpecializedData {
    Text,
    Gallery { img_urls: Vec<Url> },
    Image { img_url: Url },
    Video { video_url: Url },
}

#[derive(Clone, Debug)]
pub struct RedditPost {
    common: RedditPostCommonData,
    specialized: RedditPostSpecializedData,
}

fn manual_embed(
    author: &str,
    post: &RedditPostCommonData,
    embed_urls: &[Url],
    discord_comment: Option<&str>,
) -> String {
    let discord_comment = discord_comment
        .map(|c| {
            format!(
                "**Comment By {author}:**\n{comment}\n\n",
                author = author,
                comment = c
            )
        })
        .unwrap_or_default();

    let reddit_comment = post
        .comment
        .as_ref()
        .map(|c| {
            format!(
                "**Comment By Reddit User '{author}':**\n{comment}\n\n",
                author = c.author,
                comment = escape_markdown(&c.body)
            )
        })
        .unwrap_or_default();

    let urls = embed_urls
        .iter()
        .map(Url::as_str)
        .intersperse("\n")
        .collect::<String>();

    format!(
        ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{discord_comment}{reddit_comment}{title}\n\n{text}",
        author = author,
        src = &post.src,
        embed_url = urls,
        title = fmt_title(post),
        text = limit_descr_len(&escape_markdown(&post.text)),
        discord_comment = discord_comment,
        reddit_comment = reddit_comment,
    )
}

impl Post for RedditPost {
    fn should_embed(&self) -> bool {
        true
    }

    fn create_embed(&self, u: &User, comment: Option<&str>, create_msg: &mut CreateMessage) {
        match self.common.show_mode {
            RedditPostShowMode::Nsfw => create_msg.embed(|e| {
                e.title(fmt_title(&self.common))
                    .description("Warning NSFW: Click to view content")
                    .author(|a| a.name(&u.name))
                    .url(&self.common.src);

                if let Some(comment) = comment {
                    include_author_comment(e, u, comment);
                }

                e
            }),
            RedditPostShowMode::Spoiler => create_msg.embed(|e| {
                e.title(fmt_title(&self.common))
                    .description("Spoiler: Click to view content")
                    .author(|a| a.name(&u.name))
                    .url(&self.common.src);

                if let Some(comment) = comment {
                    include_author_comment(e, u, comment);
                }

                if let Some(comment) = &self.common.comment {
                    include_comment(e, comment);
                }

                e
            }),

            RedditPostShowMode::Default => {
                match &self.specialized {
                    RedditPostSpecializedData::Text => {
                        create_msg.embed(|e| base_embed(e, u, comment, self))
                    }
                    RedditPostSpecializedData::Image { img_url } => {
                        create_msg.embed(|e| base_embed(e, u, comment, self).image(&img_url))
                    }
                    RedditPostSpecializedData::Gallery { img_urls } => {
                        create_msg.content(manual_embed(&u.name, &self.common, &img_urls, comment))
                    }
                    RedditPostSpecializedData::Video { video_url } => create_msg.content(
                        manual_embed(&u.name, &self.common, &[video_url.clone()], comment),
                    ),
                }
            }
        };
    }
}

#[derive(Default)]
pub struct RedditAPI;

#[async_trait]
impl PostScraper for RedditAPI {
    fn is_suitable(&self, url: &Url) -> bool {
        ["reddit.com", "www.reddit.com"]
            .map(Some)
            .contains(&url.domain())
    }

    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error> {
        let (url, json) = {
            let mut u = url;
            u.set_query(None);

            let mut get_url = u.clone();
            get_url.set_path(&format!("{}.json", u.path()));

            (u, wget_json(get_url, USER_AGENT).await?)
        };

        let top_level_post = json
            .get(0)?
            .get("data")?
            .get("children")?
            .get(0)?
            .get("data")?
            .as_object()?;

        let title = top_level_post.get("title")?.as_str()?.to_string();

        let subreddit = top_level_post.get("subreddit")?.as_str()?.to_string();

        // post_json is either top_level_post or the original post (in case of crosspost)
        let (is_xpost, post_json) = top_level_post
            .get("crosspost_parent_list")
            .and_then(|arr| arr.get(0))
            .and_then(Value::as_object)
            .map(|parent| (true, parent))
            .unwrap_or((false, top_level_post));

        let original_subreddit = post_json.get("subreddit")?.as_str()?.to_string();

        let text = unescape_html(post_json.get("selftext")?.as_str()?);

        let flair = post_json
            .get("link_flair_text")
            .and_then(Value::as_str)
            .map(ToString::to_string)
            .unwrap_or_default();

        let nsfw = post_json
            .get("over_18")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let spoiler = post_json
            .get("spoiler")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let comment = {
            let comment_json = json
                .get(1)
                .and_then(|j| j.get("data"))
                .and_then(|j| j.get("children"))
                .and_then(|j| j.get(0))
                .and_then(|j| j.get("data"));

            match comment_json {
                Some(comment) if url_path_ends_with(&url, comment.get("id")?.as_str()?) => {
                    Some(RedditComment {
                        author: comment.get("author")?.as_str()?.to_owned(),
                        body: unescape_html(comment.get("body")?.as_str()?),
                    })
                }
                _ => None,
            }
        };

        let common_data = RedditPostCommonData {
            src: url,

            subreddit: if is_xpost {
                RedditPostOrigin::Crossposted {
                    from: original_subreddit,
                    to: subreddit,
                }
            } else {
                RedditPostOrigin::JustSubreddit(subreddit)
            },

            show_mode: if nsfw {
                RedditPostShowMode::Nsfw
            } else if spoiler {
                RedditPostShowMode::Spoiler
            } else {
                RedditPostShowMode::Default
            },

            title,
            text,
            flair,
            comment,
        };

        // embed_url can be "default" when the original post (referenced by crosspost) is deleted
        let alt_embed_url: Result<Url, _> = top_level_post
            .get("thumbnail")
            .and_then(Value::as_str)
            .ok_or(Error::JSONNavErr)
            .and_then(|s| Url::parse(s).map_err(Into::into));

        let specialized_data = match post_json.get("secure_media") {
            Some(Value::Object(sm)) if sm.contains_key("reddit_video") => {
                RedditPostSpecializedData::Video {
                    video_url: sm
                        .get("reddit_video")?
                        .get("fallback_url")?
                        .as_str()?
                        .try_into()?,
                }
            }

            Some(Value::Object(sm)) if sm.contains_key("oembed") => {
                RedditPostSpecializedData::Image {
                    img_url: sm
                        .get("oembed")?
                        .get("thumbnail_url")?
                        .as_str()?
                        .try_into()
                        .unwrap_or(alt_embed_url?),
                }
            }

            _ => match post_json.get("media_metadata") {
                Some(Value::Object(meta)) => RedditPostSpecializedData::Gallery {
                    img_urls: meta
                        .iter()
                        .map(|(_key, imgmeta)| {
                            imgmeta
                                .get("s")
                                .and_then(Value::as_object)
                                .and_then(|inner| inner.get("u"))
                                .and_then(Value::as_str)
                                .map(unescape_url)
                                .and_then(|u| Url::parse(&u).ok())
                        })
                        .collect::<Option<Vec<_>>>()?,
                },
                _ => {
                    let url = Url::parse(post_json.get("url")?.as_str()?).or(alt_embed_url);

                    match url {
                        Ok(url) if url_path_ends_with_image_extension(&url) => {
                            RedditPostSpecializedData::Image { img_url: url }
                        }
                        Ok(url) if url_path_ends_with(&url, ".gifv") => {
                            RedditPostSpecializedData::Video { video_url: url }
                        }
                        _ => RedditPostSpecializedData::Text,
                    }
                }
            },
        };

        Ok(Box::new(RedditPost {
            common: common_data,
            specialized: specialized_data,
        }))
    }
}
