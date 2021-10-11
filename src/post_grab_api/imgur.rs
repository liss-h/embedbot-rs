use super::*;
use scraper::selector::Selector;
use serenity::async_trait;

fn fmt_title(p: &ImgurPost) -> String {
    let em = escape_markdown(&p.title);
    let title = limit_len(&em, EMBED_TITLE_MAX_LEN - 14); // -14 for formatting

    format!("'{}' - **imgur**", title)
}

#[derive(Clone, Debug)]
pub struct ImgurPost {
    src: String,
    title: String,
    embed_url: String,
}

#[async_trait]
impl Post for ImgurPost {
    fn should_embed(&self) -> bool {
        true
    }

    async fn send_embed(
        &self,
        u: &User,
        _comment: Option<&str>,
        chan: &ChannelId,
        ctx: &Context,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        let msg = chan
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title(&fmt_title(self))
                        .author(|a| a.name(&u.name))
                        .url(&self.src)
                        .image(&self.embed_url)
                })
            })
            .await?;

        Ok(msg)
    }
}

#[derive(Default)]
pub struct ImgurAPI;

// TODO: fix; probably broken
#[async_trait]
impl PostScraper for ImgurAPI {
    fn is_suitable(&self, url: &Url) -> bool {
        match url.domain() {
            Some(d) => d.contains("imgur.com"),
            None => false,
        }
    }

    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error> {
        let html = wget_html(url.clone(), USER_AGENT).await?;

        let title_selector = Selector::parse("title").unwrap();
        let img_selector = Selector::parse(r#"link[rel="image_src"]"#).unwrap();

        let title = {
            let tmp: String = html
                .select(&title_selector)
                .next()
                .ok_or(Error::JsonNav("could not find title"))?
                .text()
                .collect();

            let beg = tmp.find(|ch: char| !ch.is_whitespace()).unwrap_or(0);

            tmp[beg..(tmp.len() - 8)].to_string()
        };

        let embed_url = html
            .select(&img_selector)
            .next()
            .ok_or(Error::JsonNav("could not find imgur url"))?
            .value()
            .attr("href")
            .ok_or(Error::JsonNav("missing href"))?
            .to_string();

        Ok(Box::new(ImgurPost {
            src: url.to_string(),
            title,
            embed_url,
        }))
    }
}
