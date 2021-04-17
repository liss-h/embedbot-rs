use serenity::async_trait;
use serenity::model::user::User;

use super::*;

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

impl Post for NineGagPost {
    fn should_embed(&self) -> bool {
        self.post_type != NineGagPostType::Image
    }

    fn create_embed(&self, u: &User, _comment: Option<&str>, create_msg: &mut CreateMessage) {
        match self.post_type {
            NineGagPostType::Image => {
                create_msg.embed(|e| e.title(&self.title).url(&self.src).image(&self.embed_url))
            }
            NineGagPostType::Video => create_msg.content(format!(
                ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{title}",
                author = u.name,
                src = &self.src,
                embed_url = self.embed_url,
                title = fmt_title(self),
            )),
        };
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
            html.select(&title_selector).next()?.text().collect()
        };

        let build_json: serde_json::Value = {
            let script_selector = scraper::Selector::parse("script").unwrap();

            let script_text: String = html
                .select(&script_selector)
                .find(|elem| elem.text().collect::<String>().contains("JSON.parse"))?
                .text()
                .collect::<String>()
                .replace("\\", "");

            serde_json::from_str(&script_text[29..(script_text.len() - 3)])?
        };

        let post_json = build_json.get("data")?.get("post")?.as_object()?;

        let (post_type, embed_url) = match post_json.get("type")?.as_str()? {
            "Photo" => (
                NineGagPostType::Image,
                post_json
                    .get("images")?
                    .get("image700")?
                    .get("url")?
                    .as_str()?
                    .to_string(),
            ),

            "Animated" => {
                let imgs = post_json.get("images")?.as_object()?;

                (
                    NineGagPostType::Video,
                    imgs.get("image460svwm")
                        .or_else(|| imgs.get("image460sv"))?
                        .get("url")?
                        .as_str()?
                        .to_string(),
                )
            }

            _ => (
                NineGagPostType::Video,
                post_json.get("vp9Url")?.as_str()?.to_string(),
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
