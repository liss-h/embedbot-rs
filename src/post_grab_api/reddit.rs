use serde_json::Value;
use serenity::async_trait;
use serenity::builder::CreateEmbed;

use crate::nav_json;

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
                from = from,
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

fn unescape_url(url: &str) -> String {
    url.replace("&amp;", "&")
}

fn unescape_html(html: &str) -> String {
    html.replace("&amp;", "&")
        .replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&quot;", "\"")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedditPostOrigin {
    JustSubreddit(String),
    Crossposted { from: String, to: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedditComment {
    author: String,
    body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedditPostShowMode {
    Default,
    Nsfw,
    Spoiler,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedditPostCommonData {
    src: Url,
    subreddit: RedditPostOrigin,
    title: String,
    text: String,
    flair: String,
    show_mode: RedditPostShowMode,
    comment: Option<RedditComment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RedditPostSpecializedData {
    Text,
    Gallery { img_urls: Vec<Url> },
    Image { img_url: Url },
    Video { video_url: Url },
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[async_trait]
impl Post for RedditPost {
    fn should_embed(&self) -> bool {
        true
    }

    async fn send_embed(
        &self,
        u: &User,
        comment: Option<&str>,
        chan: &ChannelId,
        ctx: &Context,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        let msg = chan
            .send_message(ctx, |m| match self.common.show_mode {
                RedditPostShowMode::Nsfw => m.embed(|e| {
                    e.title(fmt_title(&self.common))
                        .description("Warning NSFW: Click to view content")
                        .author(|a| a.name(&u.name))
                        .url(&self.common.src);

                    if let Some(comment) = comment {
                        include_author_comment(e, u, comment);
                    }

                    e
                }),
                RedditPostShowMode::Spoiler => m.embed(|e| {
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

                RedditPostShowMode::Default => match &self.specialized {
                    RedditPostSpecializedData::Text => m.embed(|e| base_embed(e, u, comment, self)),
                    RedditPostSpecializedData::Image { img_url } => {
                        m.embed(|e| base_embed(e, u, comment, self).image(&img_url))
                    }
                    RedditPostSpecializedData::Gallery { img_urls } => {
                        m.content(manual_embed(&u.name, &self.common, img_urls, comment))
                    }
                    RedditPostSpecializedData::Video { video_url } => m.content(manual_embed(
                        &u.name,
                        &self.common,
                        &[video_url.clone()],
                        comment,
                    )),
                },
            })
            .await?;

        Ok(msg)
    }
}

#[derive(Default)]
pub struct RedditAPI;

impl RedditAPI {
    fn analyze_post(&self, url: Url, json: Value) -> Result<RedditPost, Error> {
        let top_level_post = nav_json! {
            json => 0 => "data" => "children" => 0 => "data";
            as object
        }?;

        let title = nav_json! { top_level_post => "title"; as str }?.to_string();

        let subreddit = nav_json! { top_level_post => "subreddit"; as str }?.to_string();

        // post_json is either top_level_post or the original post (in case of crosspost)
        let (is_xpost, post_json) =
            nav_json! { top_level_post => "crosspost_parent_list" => 0; as object }
                .map(|parent| (true, parent))
                .unwrap_or((false, top_level_post));

        let original_subreddit = nav_json! { post_json => "subreddit"; as str }?.to_string();

        let text = unescape_html(nav_json! { post_json => "selftext"; as str }?);

        let flair = nav_json! { post_json => "link_flair_text"; as str }
            .map(ToString::to_string)
            .unwrap_or_default();

        let nsfw = nav_json! { post_json => "over_18"; as bool }.unwrap_or_default();

        let spoiler = nav_json! { post_json => "spoiler"; as bool }.unwrap_or_default();

        let comment = {
            let comment_json = nav_json! {
                json => 1 => "data" => "children" => 0 => "data"
            };

            match comment_json {
                Ok(comment) if url_path_ends_with(&url, nav_json! { comment => "id"; as str }?) => {
                    Some(RedditComment {
                        author: nav_json! { comment => "author"; as str }?.to_owned(),
                        body: unescape_html(nav_json! { comment => "body"; as str }?),
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
        let alt_embed_url = nav_json! { top_level_post => "thumbnail"; as str }
            .and_then(|s| Url::parse(s).map_err(Into::into));

        let specialized_data = match post_json.get("secure_media") {
            Some(Value::Object(sm)) if sm.contains_key("reddit_video") => {
                RedditPostSpecializedData::Video {
                    video_url: nav_json! { sm => "reddit_video" => "fallback_url"; as str }?
                        .try_into()?,
                }
            }

            Some(Value::Object(sm)) if sm.contains_key("oembed") => {
                RedditPostSpecializedData::Image {
                    img_url: nav_json! { sm => "oembed" => "thumbnail_url"; as str }?
                        .try_into()
                        .unwrap_or(alt_embed_url?),
                }
            }

            _ => match post_json.get("media_metadata") {
                Some(Value::Object(meta)) => {
                    let mut urls = meta
                        .iter()
                        .map(|(_key, imgmeta)| {
                            (nav_json! { imgmeta => "s" => "u"; as str })
                                .map(unescape_url)
                                .and_then(|u| Url::parse(&u).map_err(Into::into))
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| Error::JsonConv("invalid url in gallery"))?;

                    if urls.len() == 1 {
                        RedditPostSpecializedData::Image {
                            img_url: urls.pop().unwrap(),
                        }
                    } else {
                        RedditPostSpecializedData::Gallery { img_urls: urls }
                    }
                }
                _ => {
                    let url =
                        Url::parse(nav_json! { post_json => "url"; as str }?).or(alt_embed_url);

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

        Ok(RedditPost {
            common: common_data,
            specialized: specialized_data,
        })
    }

    async fn scrape_post(&self, url: Url) -> Result<RedditPost, Error> {
        let (url, json) = {
            let mut u = url;
            u.set_query(None);

            let mut get_url = u.clone();
            get_url.set_path(&format!("{}.json", u.path()));

            (u, wget_json(get_url, USER_AGENT).await?)
        };

        self.analyze_post(url, json)
    }
}

#[async_trait]
impl PostScraper for RedditAPI {
    fn is_suitable(&self, url: &Url) -> bool {
        ["reddit.com", "www.reddit.com"]
            .map(Some)
            .contains(&url.domain())
    }

    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error> {
        Ok(Box::new(self.scrape_post(url).await?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn image_post() {
        const JSON: &str = include_str!("../../test_data/reddit/image.json");
        let json: Value = serde_json::from_str(JSON).unwrap();

        let api = RedditAPI::default();
        let url = "https://www.reddit.com/r/Awwducational/comments/oi687m/a_very_rare_irrawaddy_dolphin_only_92_are/";
        let post = api.analyze_post(Url::from_str(url).unwrap(), json).unwrap();

        let expected = RedditPost {
            common: RedditPostCommonData {
                src: Url::from_str("https://www.reddit.com/r/Awwducational/comments/oi687m/a_very_rare_irrawaddy_dolphin_only_92_are/").unwrap(),
                subreddit: RedditPostOrigin::JustSubreddit("Awwducational".to_owned()),
                title: "A very rare Irrawaddy Dolphin, only 92 are estimated to still exist. These dolphins have a bulging forehead, short beak, and 12-19 teeth on each side of both jaws.".to_owned(),
                text: "".to_owned(),
                flair: "Not yet verified".to_owned(),
                show_mode: RedditPostShowMode::Default,
                comment: None,
            },
            specialized: RedditPostSpecializedData::Image {
                img_url: Url::from_str("https://i.redd.it/bsp1l1vynla71.jpg").unwrap(),
            },
        };

        assert_eq!(expected, post);
    }

    #[tokio::test]
    async fn video_post() {
        const JSON: &str = include_str!("../../test_data/reddit/video.json");
        let json: Value = serde_json::from_str(JSON).unwrap();

        let api = RedditAPI::default();
        let url = "https://www.reddit.com/r/aww/comments/oi6lfk/mama_cat_wants_her_kitten_to_be_friends_with/";
        let post = api.analyze_post(Url::from_str(url).unwrap(), json).unwrap();

        let expected = RedditPost {
            common: RedditPostCommonData {
                src: Url::from_str("https://www.reddit.com/r/aww/comments/oi6lfk/mama_cat_wants_her_kitten_to_be_friends_with/").unwrap(),
                subreddit: RedditPostOrigin::JustSubreddit(
                    "aww".to_owned(),
                ),
                title: "Mama cat wants her kitten to be friends with human baby.".to_owned(),
                text: "".to_owned(),
                flair: "".to_owned(),
                show_mode: RedditPostShowMode::Default,
                comment: None,
            },
            specialized: RedditPostSpecializedData::Video {
                video_url: Url::from_str("https://v.redd.it/jx4ua6lirla71/DASH_1080.mp4?source=fallback").unwrap(),
            }
        };

        assert_eq!(expected, post);
    }

    #[tokio::test]
    async fn gallery_post() {
        const JSON: &str = include_str!("../../test_data/reddit/gallery.json");
        let json: Value = serde_json::from_str(JSON).unwrap();

        let api = RedditAPI::default();
        let url = "https://www.reddit.com/r/watercooling/comments/ohvv5w/lian_li_o11d_xl_with_2x_3090_sli_triple_radiator/";
        let post = api.analyze_post(Url::from_str(url).unwrap(), json).unwrap();

        let expected = RedditPost {
            common: RedditPostCommonData {
                src: Url::from_str("https://www.reddit.com/r/watercooling/comments/ohvv5w/lian_li_o11d_xl_with_2x_3090_sli_triple_radiator/").unwrap(),
                subreddit: RedditPostOrigin::JustSubreddit("watercooling".to_owned()),
                title: "Lian li o11D XL with 2x 3090 SLI triple radiator. done for now will upgrade the motherboard and cpu to threadripper in future. this case is solid!".to_owned(),
                text: "".to_owned(),
                flair: "Build Complete".to_owned(),
                show_mode: RedditPostShowMode::Default,
                comment: None,
            },
            specialized: RedditPostSpecializedData::Gallery {
                img_urls: vec![
                    Url::from_str("https://preview.redd.it/nuwtn1ytsha71.jpg?width=3876&format=pjpg&auto=webp&s=7743bf4c3dbdff8e34c5a0a33d5171e4b485e1e5").unwrap(),
                    Url::from_str("https://preview.redd.it/wrro81ytsha71.jpg?width=4000&format=pjpg&auto=webp&s=5f1a86f3783d7ae290f733083b2af4397332c1be").unwrap(),
                ],
            },
        };

        assert_eq!(expected, post);
    }
}
