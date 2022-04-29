#![cfg(feature = "imgur")]

use super::{
    escape_markdown, limit_len, wget_html, CreateResponse, EmbedOptions, Error, Post as PostTrait, PostScraper,
    EMBED_TITLE_MAX_LEN,
};
use crate::post_grab_api::include_author_comment;
use scraper::selector::Selector;
use serde::{Deserialize, Serialize};
use serenity::{async_trait, model::user::User};
use url::Url;

fn fmt_title(p: &Post) -> String {
    let em = escape_markdown(&p.title);
    let title = limit_len(&em, EMBED_TITLE_MAX_LEN - 14); // -14 for formatting

    format!("'{}' - **imgur**", title)
}

#[derive(Clone, Debug)]
pub struct Post {
    src: String,
    title: String,
    embed_url: String,
}

impl PostTrait for Post {
    fn create_embed<'data>(&'data self, u: &User, opts: &EmbedOptions, response: CreateResponse<'_, 'data>) {
        response.embed(|e| {
            e.title(&fmt_title(self))
                .author(|a| a.name(&u.name))
                .url(&self.src)
                .image(&self.embed_url);

            if let Some(comment) = &opts.comment {
                include_author_comment(e, u, comment);
            }

            e
        });
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ApiSettings {}

pub struct Api {
    pub settings: ApiSettings,
}

// TODO: fix; probably broken
#[async_trait]
impl PostScraper for Api {
    type Output = Post;

    fn is_suitable(&self, url: &Url) -> bool {
        match url.domain() {
            Some(d) => d.contains("imgur.com"),
            None => false,
        }
    }

    fn should_embed(&self, _post: &Self::Output) -> bool {
        true
    }

    async fn get_post(&self, url: Url) -> Result<Self::Output, Error> {
        let html = wget_html(url.clone()).await?;

        let title_selector = Selector::parse("title").unwrap();
        let img_selector = Selector::parse(r#"link[rel="image_src"]"#).unwrap();

        let title = {
            let tmp: String = html
                .select(&title_selector)
                .next()
                .ok_or_else(|| Error::Navigation("could not find title".to_owned()))?
                .text()
                .collect();

            let beg = tmp.find(|ch: char| !ch.is_whitespace()).unwrap_or(0);

            tmp[beg..(tmp.len() - 8)].to_string()
        };

        let embed_url = html
            .select(&img_selector)
            .next()
            .ok_or_else(|| Error::Navigation("could not find imgur url".to_owned()))?
            .value()
            .attr("href")
            .ok_or_else(|| Error::Navigation("missing href".to_owned()))?
            .to_string();

        Ok(Post {
            src: url.to_string(),
            title,
            embed_url,
        })
    }
}
