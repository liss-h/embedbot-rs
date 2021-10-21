#![cfg(feature = "ninegag")]

use super::{
    escape_markdown, limit_len, wget_html, Error, Post, PostScraper, Settings, EMBED_TITLE_MAX_LEN,
    USER_AGENT,
};
use crate::{embed_bot::PostType, nav_json};
use serenity::{
    async_trait,
    client::Context,
    model::{channel::Message, id::ChannelId, user::User},
};
use url::Url;

fn fmt_title(p: &NineGagPost) -> String {
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
pub struct NineGagPost {
    src: String,
    title: String,
    embed_url: String,
    post_type: NineGagPostType,
}

#[async_trait]
impl Post for NineGagPost {
    fn should_embed(&self, settings: &Settings) -> bool {
        settings
            .embed_settings
            .ninegag
            .0
            .contains(&match self.post_type {
                NineGagPostType::Video => PostType::Video,
                NineGagPostType::Image => PostType::Image,
            })
    }

    async fn send_embed(
        &self,
        u: &User,
        comment: Option<&str>,
        chan: ChannelId,
        ctx: &Context,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        let msg = chan.send_message(ctx, |m| {
            match self.post_type {
                NineGagPostType::Image => {
                    m.embed(|e| e.title(&self.title).url(&self.src).image(&self.embed_url))
                }
                NineGagPostType::Video => {
                    let discord_comment = comment
                        .map(|c| {
                            format!(
                                "**Comment By {author}:**\n{comment}\n\n",
                                author = u.name,
                                comment = c
                            )
                        })
                        .unwrap_or_default();

                    m.content(format!(
                        ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{discord_comment}{title}",
                        author = u.name,
                        src = &self.src,
                        embed_url = self.embed_url,
                        discord_comment = discord_comment,
                        title = fmt_title(self),
                    ))
                }
            }
        })
        .await?;

        Ok(msg)
    }
}

#[derive(Default)]
pub struct NineGagAPI;

#[async_trait]
impl PostScraper for NineGagAPI {
    fn is_suitable(&self, url: &Url) -> bool {
        url.domain() == Some("9gag.com")
    }

    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error> {
        let html = wget_html(url.clone(), USER_AGENT).await?;

        let title: String = {
            let title_selector = scraper::Selector::parse("title").unwrap();
            html.select(&title_selector)
                .next()
                .ok_or(Error::JsonNav("could not find title"))?
                .text()
                .collect()
        };

        let build_json: serde_json::Value = {
            let script_selector = scraper::Selector::parse("script").unwrap();

            let script_text: String = html
                .select(&script_selector)
                .find(|elem| elem.text().collect::<String>().contains("JSON.parse"))
                .ok_or(Error::JsonNav("could not find json"))?
                .text()
                .collect::<String>()
                .replace("\\", "");

            serde_json::from_str(&script_text[29..(script_text.len() - 3)])?
        };

        let post_json = nav_json! { build_json => "data" => "post"; as object }?;

        let (post_type, embed_url) = match nav_json! { post_json => "type"; as str }? {
            "Photo" => (
                NineGagPostType::Image,
                nav_json! { post_json => "images" => "image700" => "url"; as str }?.to_string(),
            ),

            "Animated" => {
                let imgs = nav_json! { post_json => "images"; as object }?;

                let img_alts = nav_json! { imgs => "image460svwm" }
                    .or_else(|_| nav_json! { imgs => "image460sv" })?;

                (
                    NineGagPostType::Video,
                    nav_json! { img_alts => "url"; as str }?.to_string(),
                )
            }

            _ => (
                NineGagPostType::Video,
                nav_json! { post_json => "vp9Url"; as str }?.to_string(),
            ),
        };

        Ok(Box::new(NineGagPost {
            src: url.to_string(),
            title: title[0..(title.len() - 7)].to_string(), // remove ' - 9GAG' from end
            embed_url,
            post_type,
        }))
    }
}
