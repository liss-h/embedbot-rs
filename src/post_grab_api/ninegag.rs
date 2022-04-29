#![cfg(feature = "ninegag")]

use super::{
    escape_markdown, limit_len, wget_html, CreateResponse, EmbedOptions, Error, Post as PostTrait, PostScraper,
    EMBED_TITLE_MAX_LEN,
};
use crate::post_grab_api::include_author_comment;
use json_nav::json_nav;
use serde::{Deserialize, Serialize};
use serenity::{async_trait, model::user::User};
use std::collections::HashSet;
use url::Url;

fn fmt_title(p: &Post) -> String {
    let em = escape_markdown(&p.title);
    let title = limit_len(&em, EMBED_TITLE_MAX_LEN - 12); // -12 for formatting

    format!("'{}' - **9GAG**", title)
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum NineGagPostType {
    Image,
    Video,
}

#[derive(Clone, Debug)]
pub struct Post {
    src: String,
    title: String,
    embed_url: String,
    post_type: NineGagPostType,
}

impl PostTrait for Post {
    fn create_embed<'data>(&'data self, u: &User, opts: &EmbedOptions, response: CreateResponse<'_, 'data>) {
        match self.post_type {
            NineGagPostType::Image => {
                response.embed(|e| {
                    e.title(&self.title).url(&self.src).image(&self.embed_url);

                    if let Some(comment) = &opts.comment {
                        include_author_comment(e, u, comment);
                    }

                    e
                });
            }
            NineGagPostType::Video => {
                let discord_comment = opts
                    .comment
                    .as_ref()
                    .map(|c| format!("**Comment By {author}:**\n{comment}\n\n", author = u.name, comment = c))
                    .unwrap_or_default();

                response.content(format!(
                    ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{discord_comment}{title}",
                    author = u.name,
                    src = &self.src,
                    embed_url = self.embed_url,
                    discord_comment = discord_comment,
                    title = fmt_title(self),
                ));
            }
        }
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Debug)]
pub enum SettingsPostType {
    Image,
    Video,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiSettings {
    pub embed_set: HashSet<SettingsPostType>,
}

pub struct Api {
    pub settings: ApiSettings,
}

#[async_trait]
impl PostScraper for Api {
    type Output = Post;

    fn is_suitable(&self, url: &Url) -> bool {
        url.domain() == Some("9gag.com")
    }

    fn should_embed(&self, post: &Self::Output) -> bool {
        self.settings.embed_set.contains(&match post.post_type {
            NineGagPostType::Video => SettingsPostType::Video,
            NineGagPostType::Image => SettingsPostType::Image,
        })
    }

    async fn get_post(&self, url: Url) -> Result<Self::Output, Error> {
        let html = wget_html(url.clone()).await?;

        let title: String = {
            let title_selector = scraper::Selector::parse("title").unwrap();
            html.select(&title_selector)
                .next()
                .ok_or_else(|| Error::Navigation("could not find title".to_owned()))?
                .text()
                .collect()
        };

        let build_json: serde_json::Value = {
            let script_selector = scraper::Selector::parse("script").unwrap();

            let script_text: String = html
                .select(&script_selector)
                .find(|elem| elem.text().collect::<String>().contains("JSON.parse"))
                .ok_or_else(|| Error::Navigation("could not find json".to_owned()))?
                .text()
                .collect::<String>()
                .replace('\\', "");

            serde_json::from_str(&script_text[29..(script_text.len() - 3)])?
        };

        let post_json = json_nav! { build_json => "data" => "post"; as object }?;

        let (post_type, embed_url) = match json_nav! { post_json => "type"; as str }? {
            "Photo" => (
                NineGagPostType::Image,
                json_nav! { post_json => "images" => "image700" => "url"; as str }?.to_string(),
            ),

            "Animated" => {
                let imgs = json_nav! { post_json => "images"; as object }?;

                let img_alts = json_nav! { imgs => "image460svwm" }.or_else(|_| json_nav! { imgs => "image460sv" })?;

                (
                    NineGagPostType::Video,
                    json_nav! { img_alts => "url"; as str }?.to_string(),
                )
            }

            _ => (
                NineGagPostType::Video,
                json_nav! { post_json => "vp9Url"; as str }?.to_string(),
            ),
        };

        Ok(Post {
            src: url.to_string(),
            title: title[0..(title.len() - 7)].to_string(), // remove ' - 9GAG' from end
            embed_url,
            post_type,
        })
    }
}
