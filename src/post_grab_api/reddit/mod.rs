#![cfg(feature = "reddit")]

pub mod module_settings;

use super::{
    escape_markdown, include_author_comment, limit_descr_len, limit_len, url_path_ends_with,
    url_path_ends_with_image_extension, wget, wget_json, CreateResponse, EmbedOptions, Post as PostTrait, PostScraper,
    EMBED_TITLE_MAX_LEN,
};
use itertools::Itertools;
use json_nav::json_nav;
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateEmbedAuthor},
    model::user::User,
};
use std::convert::TryInto;
use url::Url;

async fn find_canonical_post_url<U: IntoUrl>(post_url: U) -> anyhow::Result<Url> {
    let url = post_url.into_url()?;

    match wget(url.clone()).await {
        Ok(resp) if resp.url().path() != "/over18" => Ok(resp.url().to_owned()),
        _ => Ok(url),
    }
}

fn fmt_title(p: &PostCommonData) -> String {
    let flair = (!p.flair.is_empty())
        .then(|| format!("[{}] ", p.flair))
        .unwrap_or_default();

    match &p.subreddit {
        PostOrigin::Crossposted { from, to } => {
            let em = escape_markdown(&p.title);
            let title = limit_len(&em, EMBED_TITLE_MAX_LEN - 34 - 18 - to.len() - flair.len() - from.len()); // -34-18 for formatting

            format!(
                "'{title}' {flair}- **reddit.com/r/{subreddit}\n[XPosted from r/{from}]**",
                title = title,
                flair = flair,
                subreddit = to,
                from = from,
            )
        },
        PostOrigin::JustSubreddit(subreddit) => {
            let em = escape_markdown(&p.title);
            let title = limit_len(&em, EMBED_TITLE_MAX_LEN - 34 - subreddit.len() - flair.len()); // -34 for formatting

            format!(
                "'{title}' {flair}- **reddit.com/r/{subreddit}**",
                title = title,
                flair = flair,
                subreddit = subreddit,
            )
        },
    }
}

fn base_embed(e: CreateEmbed, u: &User, comment: Option<&str>, post: &PostCommonData) -> CreateEmbed {
    let mut e = e
        .title(fmt_title(post))
        .description(limit_descr_len(&post.text))
        .author(CreateEmbedAuthor::new(&u.name))
        .url(post.src.as_str());

    if let Some(comment) = comment {
        e = include_author_comment(e, u, comment);
    }

    if let Some(comment) = &post.comment {
        e = include_comment(e, comment);
    }

    e
}

fn include_comment(e: CreateEmbed, comment: &Comment) -> CreateEmbed {
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
pub enum PostOrigin {
    JustSubreddit(String),
    Crossposted { from: String, to: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    author: String,
    body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostCommonData {
    src: Url,
    subreddit: PostOrigin,
    title: String,
    text: String,
    flair: String,
    nsfw: bool,
    spoiler: bool,
    comment: Option<Comment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostSpecializedData {
    Text,
    Gallery { img_urls: Vec<Url> },
    Image { img_url: Url },
    Video { video_url: Url },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Post {
    common: PostCommonData,
    specialized: PostSpecializedData,
}

fn manual_embed(author: &str, post: &PostCommonData, embed_urls: &[Url], discord_comment: Option<&str>) -> String {
    let discord_comment = discord_comment
        .map(|c| format!("**Comment By {author}:**\n{comment}\n\n", author = author, comment = c))
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

    let urls = Itertools::intersperse(embed_urls.iter().map(Url::as_str), "\n").collect::<String>();

    format!(
        ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{discord_comment}{reddit_comment}{title}\n\n{text}",
        author = author,
        src = &post.src,
        embed_url = urls,
        title = fmt_title(post),
        text = limit_descr_len(&post.text),
        discord_comment = discord_comment,
        reddit_comment = reddit_comment,
    )
}

impl PostTrait for Post {
    fn create_embed(&self, u: &User, opts: &EmbedOptions, response: CreateResponse) -> CreateResponse {
        if self.common.nsfw && !opts.ignore_nsfw {
            response.embed({
                let mut e = CreateEmbed::new()
                    .title(fmt_title(&self.common))
                    .description("Warning NSFW: Click to view content")
                    .author(CreateEmbedAuthor::new(&u.name))
                    .url(self.common.src.as_str());

                if let Some(comment) = &opts.comment {
                    e = include_author_comment(e, u, comment);
                }

                e
            })
        } else if self.common.spoiler && !opts.ignore_spoiler {
            response.embed({
                let mut e = CreateEmbed::new()
                    .title(fmt_title(&self.common))
                    .description("Spoiler: Click to view content")
                    .author(CreateEmbedAuthor::new(&u.name))
                    .url(self.common.src.as_str());

                if let Some(comment) = &opts.comment {
                    e = include_author_comment(e, u, comment);
                }

                if let Some(comment) = &self.common.comment {
                    e = include_comment(e, comment);
                }

                e
            })
        } else {
            match &self.specialized {
                PostSpecializedData::Text => {
                    response.embed(base_embed(CreateEmbed::new(), u, opts.comment.as_deref(), &self.common))
                },
                PostSpecializedData::Image { img_url } => response.embed(
                    base_embed(CreateEmbed::new(), u, opts.comment.as_deref(), &self.common).image(img_url.as_str()),
                ),
                PostSpecializedData::Gallery { img_urls } => {
                    response.content(manual_embed(&u.name, &self.common, img_urls, opts.comment.as_deref()))
                },
                PostSpecializedData::Video { video_url } => response.content(manual_embed(
                    &u.name,
                    &self.common,
                    &[video_url.clone()],
                    opts.comment.as_deref(),
                )),
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiSettings {
    pub embed_set: module_settings::EmbedSet,
}

pub struct Api {
    settings: ApiSettings,
}

impl Api {
    pub fn from_settings(settings: ApiSettings) -> Self {
        Self { settings }
    }

    fn analyze_post(url: Url, json: &Value) -> anyhow::Result<Post> {
        let top_level_post = json_nav! {
            json => 0 => "data" => "children" => 0 => "data";
            as object
        }?;

        let title = json_nav! { top_level_post => "title"; as str }?.to_string();

        let subreddit = json_nav! { top_level_post => "subreddit"; as str }?.to_string();

        // post_json is either top_level_post or the original post (in case of crosspost)
        let (is_xpost, post_json) = json_nav! { top_level_post => "crosspost_parent_list" => 0; as object }
            .map(|parent| (true, parent))
            .unwrap_or((false, top_level_post));

        let original_subreddit = json_nav! { post_json => "subreddit"; as str }?.to_string();

        let text = unescape_html(json_nav! { post_json => "selftext"; as str }?);

        let flair = json_nav! { post_json => "link_flair_text"; as str }
            .map(ToString::to_string)
            .unwrap_or_default();

        let nsfw = json_nav! { post_json => "over_18"; as bool }.unwrap_or_default();

        let spoiler = json_nav! { post_json => "spoiler"; as bool }.unwrap_or_default();

        let comment = {
            let comment_json = json_nav! {
                json => 1 => "data" => "children" => 0 => "data"
            };

            match comment_json {
                Ok(comment) if url_path_ends_with(&url, json_nav! { comment => "id"; as str }?) => Some(Comment {
                    author: json_nav! { comment => "author"; as str }?.to_owned(),
                    body: unescape_html(json_nav! { comment => "body"; as str }?),
                }),
                _ => None,
            }
        };

        let common_data = PostCommonData {
            src: url,

            subreddit: if is_xpost {
                PostOrigin::Crossposted { from: original_subreddit, to: subreddit }
            } else {
                PostOrigin::JustSubreddit(subreddit)
            },
            nsfw,
            spoiler,
            title,
            text,
            flair,
            comment,
        };

        // embed_url can be "default" when the original post (referenced by crosspost) is deleted
        let alt_embed_url = json_nav! { top_level_post => "thumbnail"; as str }
            .map_err(anyhow::Error::from)
            .and_then(|s| Url::parse(s).map_err(anyhow::Error::from));

        let specialized_data = match post_json.get("secure_media") {
            Some(Value::Object(sm)) if sm.contains_key("reddit_video") => PostSpecializedData::Video {
                video_url: json_nav! { sm => "reddit_video" => "fallback_url"; as str }?.try_into()?,
            },

            Some(Value::Object(sm)) if sm.contains_key("oembed") => PostSpecializedData::Image {
                img_url: json_nav! { sm => "oembed" => "thumbnail_url"; as str }?
                    .try_into()
                    .unwrap_or(alt_embed_url?),
            },

            _ => {
                if let Some(Value::Object(meta)) = post_json.get("media_metadata") {
                    let mut urls = meta
                        .iter()
                        .map(|(_key, imgmeta)| {
                            (json_nav! { imgmeta => "s" => "u"; as str })
                                .map(unescape_url)
                                .map_err(anyhow::Error::from)
                                .and_then(|u| Url::parse(&u).map_err(anyhow::Error::from))
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| json_nav::JsonNavError::TypeMismatch { expected: "url" })?;

                    if urls.len() == 1 {
                        PostSpecializedData::Image { img_url: urls.pop().unwrap() }
                    } else {
                        PostSpecializedData::Gallery { img_urls: urls }
                    }
                } else {
                    let url = Url::parse(json_nav! { post_json => "url"; as str }?).or(alt_embed_url);

                    match url {
                        Ok(url) if url_path_ends_with_image_extension(&url) => {
                            PostSpecializedData::Image { img_url: url }
                        },
                        Ok(url) if url_path_ends_with(&url, ".gifv") => PostSpecializedData::Video { video_url: url },
                        _ => PostSpecializedData::Text,
                    }
                }
            },
        };

        Ok(Post { common: common_data, specialized: specialized_data })
    }

    async fn scrape_post(&self, url: Url) -> anyhow::Result<Post> {
        let (url, json) = {
            let mut u = find_canonical_post_url(url).await?;
            u.set_query(None);

            let mut get_url = u.clone();
            get_url.set_path(&format!("{}.json", u.path()));

            (u, wget_json(get_url).await?)
        };

        Self::analyze_post(url, &json)
    }
}

#[async_trait]
impl PostScraper for Api {
    type Output = Post;

    fn is_suitable(&self, url: &Url) -> bool {
        ["reddit.com", "www.reddit.com"].map(Some).contains(&url.domain())
    }

    fn should_embed(&self, post: &Self::Output) -> bool {
        let content_type = match &post.specialized {
            PostSpecializedData::Text => module_settings::ContentType::Text,
            PostSpecializedData::Gallery { .. } => module_settings::ContentType::Gallery,
            PostSpecializedData::Image { .. } => module_settings::ContentType::Image,
            PostSpecializedData::Video { .. } => module_settings::ContentType::Video,
        };

        let origin_type = match &post.common.subreddit {
            PostOrigin::JustSubreddit(_) => module_settings::OriginType::NonCrossposted,
            PostOrigin::Crossposted { .. } => module_settings::OriginType::Crossposted,
        };

        let nsfw_type = if post.common.nsfw {
            module_settings::NsfwType::Nsfw
        } else {
            module_settings::NsfwType::Sfw
        };

        self.settings
            .embed_set
            .contains(&module_settings::PostClassification { content_type, origin_type, nsfw_type })
    }

    async fn get_post(&self, url: Url) -> anyhow::Result<Self::Output> {
        Ok(self.scrape_post(url).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn image_post() {
        const JSON: &str = include_str!("../../../test_data/reddit/image.json");
        let json: Value = serde_json::from_str(JSON).unwrap();

        let url = "https://www.reddit.com/r/Awwducational/comments/oi687m/a_very_rare_irrawaddy_dolphin_only_92_are/";
        let post = Api::analyze_post(Url::from_str(url).unwrap(), &json).unwrap();

        let expected = Post {
            common: PostCommonData {
                src: Url::from_str("https://www.reddit.com/r/Awwducational/comments/oi687m/a_very_rare_irrawaddy_dolphin_only_92_are/").unwrap(),
                subreddit: PostOrigin::JustSubreddit("Awwducational".to_owned()),
                title: "A very rare Irrawaddy Dolphin, only 92 are estimated to still exist. These dolphins have a bulging forehead, short beak, and 12-19 teeth on each side of both jaws.".to_owned(),
                text: "".to_owned(),
                flair: "Not yet verified".to_owned(),
                nsfw: false,
                spoiler: false,
                comment: None,
            },
            specialized: PostSpecializedData::Image {
                img_url: Url::from_str("https://i.redd.it/bsp1l1vynla71.jpg").unwrap(),
            },
        };

        assert_eq!(expected, post);
    }

    #[tokio::test]
    async fn video_post() {
        const JSON: &str = include_str!("../../../test_data/reddit/video.json");
        let json: Value = serde_json::from_str(JSON).unwrap();

        let url = "https://www.reddit.com/r/aww/comments/oi6lfk/mama_cat_wants_her_kitten_to_be_friends_with/";
        let post = Api::analyze_post(Url::from_str(url).unwrap(), &json).unwrap();

        let expected = Post {
            common: PostCommonData {
                src: Url::from_str(
                    "https://www.reddit.com/r/aww/comments/oi6lfk/mama_cat_wants_her_kitten_to_be_friends_with/",
                )
                .unwrap(),
                subreddit: PostOrigin::JustSubreddit("aww".to_owned()),
                title: "Mama cat wants her kitten to be friends with human baby.".to_owned(),
                text: "".to_owned(),
                flair: "".to_owned(),
                nsfw: false,
                spoiler: false,
                comment: None,
            },
            specialized: PostSpecializedData::Video {
                video_url: Url::from_str("https://v.redd.it/jx4ua6lirla71/DASH_1080.mp4?source=fallback").unwrap(),
            },
        };

        assert_eq!(expected, post);
    }

    #[tokio::test]
    async fn gallery_post() {
        const JSON: &str = include_str!("../../../test_data/reddit/gallery.json");
        let json: Value = serde_json::from_str(JSON).unwrap();

        let url =
            "https://www.reddit.com/r/watercooling/comments/ohvv5w/lian_li_o11d_xl_with_2x_3090_sli_triple_radiator/";
        let post = Api::analyze_post(Url::from_str(url).unwrap(), &json).unwrap();

        let expected = Post {
            common: PostCommonData {
                src: Url::from_str("https://www.reddit.com/r/watercooling/comments/ohvv5w/lian_li_o11d_xl_with_2x_3090_sli_triple_radiator/").unwrap(),
                subreddit: PostOrigin::JustSubreddit("watercooling".to_owned()),
                title: "Lian li o11D XL with 2x 3090 SLI triple radiator. done for now will upgrade the motherboard and cpu to threadripper in future. this case is solid!".to_owned(),
                text: "".to_owned(),
                flair: "Build Complete".to_owned(),
                nsfw: false,
                spoiler: false,
                comment: None,
            },
            specialized: PostSpecializedData::Gallery {
                img_urls: vec![
                    Url::from_str("https://preview.redd.it/nuwtn1ytsha71.jpg?width=3876&format=pjpg&auto=webp&s=7743bf4c3dbdff8e34c5a0a33d5171e4b485e1e5").unwrap(),
                    Url::from_str("https://preview.redd.it/wrro81ytsha71.jpg?width=4000&format=pjpg&auto=webp&s=5f1a86f3783d7ae290f733083b2af4397332c1be").unwrap(),
                ],
            },
        };

        assert_eq!(expected, post);
    }
}
