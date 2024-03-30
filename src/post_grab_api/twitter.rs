#![cfg(feature = "twitter")]

use super::{
    escape_markdown, include_author_comment, limit_descr_len, CreateResponse, EmbedOptions, Post as PostTrait,
    PostScraper,
};
use headless_chrome::LaunchOptions;
use itertools::Itertools;
use scraper::Html;
use serde::{Deserialize, Serialize};
use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter},
    model::user::User,
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use url::Url;

fn fmt_title(p: &PostCommonData) -> String {
    format!("@{author} - **twitter.com**", author = p.author)
}

fn wget_rendered_html(url: &Url, chrome_executable: Option<&Path>) -> anyhow::Result<Html> {
    let browser = headless_chrome::Browser::new(
        LaunchOptions::default_builder()
            .path(chrome_executable.map(ToOwned::to_owned))
            .build()
            .unwrap(),
    )?;

    let tab = browser.new_tab()?;
    tab.navigate_to(url.as_str())?;
    tab.wait_until_navigated()?;
    let content = tab.get_content()?;

    Ok(Html::parse_document(&content))
}

#[derive(Clone, Debug)]
pub struct PostCommonData {
    src: Url,
    author: String,
    text: String,
}

#[derive(Clone, Debug)]
pub enum PostSpecializedData {
    Text,
    Image { img_src: Vec<Url> },
    Video { video_src: Url },
    VideoPreview { thumbnail_src: Url },
}

#[derive(Clone, Debug)]
pub struct Post {
    common: PostCommonData,
    specialized: PostSpecializedData,
}

fn base_embed(e: CreateEmbed, u: &User, comment: Option<&str>, post: &PostCommonData) -> CreateEmbed {
    let mut e = e
        .author(CreateEmbedAuthor::new(&u.name))
        .title(fmt_title(post))
        .description(limit_descr_len(&post.text))
        .url(post.src.as_str());

    if let Some(comment) = comment {
        e = include_author_comment(e, u, comment);
    }

    e
}

fn manual_embed(u: &User, post: &PostCommonData, embed_urls: &[Url], discord_comment: Option<&str>) -> String {
    let discord_comment = discord_comment
        .map(|c| format!("**Comment By {author}:**\n{comment}\n\n", author = u.name, comment = c))
        .unwrap_or_default();

    let urls = Itertools::intersperse(embed_urls.iter().map(Url::as_str), "\n").collect::<String>();

    format!(
        ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{discord_comment}{title}\n\n{text}",
        author = u.name,
        src = &post.src,
        embed_url = urls,
        title = fmt_title(post),
        text = limit_descr_len(&escape_markdown(&post.text)),
    )
}

impl PostTrait for Post {
    fn create_embed(&self, u: &User, opts: &EmbedOptions, response: CreateResponse) -> CreateResponse {
        match &self.specialized {
            PostSpecializedData::Text => {
                response.embed(base_embed(CreateEmbed::new(), u, opts.comment.as_deref(), &self.common))
            },
            PostSpecializedData::Image { img_src } if img_src.len() == 1 => response.embed(
                base_embed(CreateEmbed::new(), u, opts.comment.as_deref(), &self.common).image(img_src[0].as_str()),
            ),
            PostSpecializedData::Image { img_src } => {
                response.content(manual_embed(u, &self.common, img_src, opts.comment.as_deref()))
            },
            PostSpecializedData::Video { video_src } => response.content(manual_embed(
                u,
                &self.common,
                &[video_src.clone()],
                opts.comment.as_deref(),
            )),
            PostSpecializedData::VideoPreview { thumbnail_src } => response.embed(
                base_embed(CreateEmbed::new(), u, opts.comment.as_deref(), &self.common)
                    .image(thumbnail_src.as_str())
                    .footer(CreateEmbedFooter::new(
                        "This was originally a video. Click title to watch on twitter.",
                    )),
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
pub enum SettingsPostType {
    Text,
    Image,
    Video,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiSettings {
    pub chrome_executable: Option<PathBuf>,
    pub embed_set: HashSet<SettingsPostType>,
}

pub struct Api {
    pub settings: ApiSettings,
}

#[async_trait]
impl PostScraper for Api {
    type Output = Post;

    fn is_suitable(&self, url: &Url) -> bool {
        url.domain() == Some("twitter.com") || url.domain() == Some("x.com")
    }

    fn should_embed(&self, post: &Self::Output) -> bool {
        self.settings.embed_set.contains(&match post.specialized {
            PostSpecializedData::Text => SettingsPostType::Text,
            PostSpecializedData::Image { .. } => SettingsPostType::Image,
            PostSpecializedData::Video { .. } | PostSpecializedData::VideoPreview { .. } => SettingsPostType::Video,
        })
    }

    async fn get_post(&self, url: Url) -> anyhow::Result<Self::Output> {
        let chrome_exec = self.settings.chrome_executable.clone();

        tokio::task::spawn_blocking(move || {
            let author = url
                .path_segments()
                .ok_or_else(|| anyhow::anyhow!("Url missing path"))?
                .next()
                .ok_or_else(|| anyhow::anyhow!("Url missing first path element"))?
                .to_owned();

            let html = wget_rendered_html(&url, chrome_exec.as_deref())?;

            let text = {
                let selector = scraper::Selector::parse(r#"article div[data-testid="tweetText"]"#).unwrap();

                html.select(&selector)
                    .next()
                    .map(|e| e.text().filter(|&s| s != "…").collect())
                    .unwrap_or_default()
            };

            let common = PostCommonData { text, author, src: url };

            let img_urls: Vec<_> = {
                let selector = scraper::Selector::parse(r#"article img[alt]:not([alt=""])"#).unwrap();

                html.select(&selector)
                    .filter_map(|e| e.attr("src"))
                    .filter(|src| src.starts_with("https://pbs.twimg.com/media"))
                    .filter_map(|s| Url::parse(s).ok())
                    .collect()
            };

            if !img_urls.is_empty() {
                Ok(Post { common, specialized: PostSpecializedData::Image { img_src: img_urls } })
            } else {
                let selector = scraper::Selector::parse("article video").unwrap();

                if let Some(video) = html.select(&selector).next() {
                    if matches!(video.attr("type"), Some("video/mp4")) {
                        let src = video.attr("src").unwrap();

                        Ok(Post {
                            common,
                            specialized: PostSpecializedData::Video { video_src: Url::parse(src)? },
                        })
                    } else {
                        let poster = video.attr("poster").unwrap();

                        Ok(Post {
                            common,
                            specialized: PostSpecializedData::VideoPreview { thumbnail_src: Url::parse(poster)? },
                        })
                    }
                } else {
                    Ok(Post { common, specialized: PostSpecializedData::Text })
                }
            }
        })
        .await
        .unwrap()
    }
}
