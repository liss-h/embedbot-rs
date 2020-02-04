use super::*;
use serenity::model::user::User;


fn fmt_title(p: &NineGagPost) -> String {
    let title = limit_len(
        &escape_markdown(&p.title),
        EMBED_TITLE_MAX_LEN - 12); // -12 for formatting

    format!("'{}' - **GAG**", title)
}


#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum NineGagPostType {
    Image,
    Video,
}

#[derive(Clone)]
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

    fn create_embed(&self, u: &User, create_msg: &mut CreateMessage) {
        match self.post_type {
            NineGagPostType::Image => create_msg.embed(|e| {
                e.title(&self.title)
                    .url(&self.src)
                    .image(&self.embed_url)
            }),
            NineGagPostType::Video => create_msg.content(format!(
                ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{title}",
                author = u.name,
                src = &self.src,
                embed_url = self.embed_url,
                title = fmt_title(self),
            ))
        };
    }
}


#[derive(Default)]
pub struct NineGagAPI;

impl PostGrabAPI for NineGagAPI {
    fn is_suitable(&self, url: &str) -> bool {
        url.starts_with("https://9gag.com")
    }

    fn get_post(&self, url: &str) -> Result<Box<dyn Post>, Error> {
        let html = wget_html(url, USER_AGENT)?;

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

        let post_json = build_json
            .get("data")?
            .get("post")?
            .as_object()?;

        let post_type_str = post_json.get("type")?.as_str()?;

        let embed_url = match post_type_str {
            "Photo" => post_json
                .get("images")?
                .get("image700")?
                .get("url")?
                .as_str()?,

            "Animated" => {
                let imgs = post_json
                    .get("images")?
                    .as_object()?;

                imgs.get("image460svwm")
                        .or(imgs.get("image460sv"))?
                    .get("url")?
                    .as_str()?
            },

            _ => post_json.get("vp9Url")?.as_str()?,
        }
        .to_string();

        let post_type = if post_type_str == "Photo" {
            NineGagPostType::Image
        } else {
            NineGagPostType::Video
        };

        Ok(Box::new(NineGagPost {
            src: url.to_string(),
            title: title[0..(title.len() - 7)].to_string(), // remove ' - 9GAG' from end
            embed_url,
            post_type,
        }))
    }
}
