use super::*;

use scraper::selector::Selector;

fn fmt_title(p: &ImgurPost) -> String {
    let title = limit_len(
        &escape_markdown(&p.title),
        EMBED_TITLE_MAX_LEN - 14); // -14 for formatting

    format!("'{}' - **imgur**", title)
}


#[derive(Clone, Debug)]
pub struct ImgurPost {
    src: String,
    title: String,
    embed_url: String,
}

impl Post for ImgurPost {
    fn should_embed(&self) -> bool {
        true
    }

    fn create_embed<'a>(&self, u: &User, create_msg: &mut CreateMessage) {
        create_msg.embed(|e| {
            e.title(&fmt_title(self))
                .author(|a| a.name(&u.name))
                .url(&self.src)
                .image(&self.embed_url)
        });
    }
}


#[derive(Default)]
pub struct ImgurAPI;

// TODO: fix; probably broken
impl PostGrabAPI for ImgurAPI {
    fn is_suitable(&self, url: &str) -> bool {
        url.starts_with("https://")
            && url.contains("imgur.com")
    }

    fn get_post(&self, url: &str) -> Result<Box<dyn Post>, Error> {
        let html = wget_html(url, USER_AGENT)?;

        let title_selector = Selector::parse("title").unwrap();
        let img_selector = Selector::parse(r#"link[rel="image_src"]"#).unwrap();

        let title = {
            let tmp: String = html.select(&title_selector).next()?.text().collect();
            let beg = tmp.find(|ch: char| !ch.is_whitespace())
                .unwrap_or(0);

            tmp[beg..(tmp.len() - 8)].to_string()
        };

        let embed_url = html
            .select(&img_selector)
            .next()?
            .value()
            .attr("href")?
            .to_string();

        Ok(Box::new(ImgurPost {
            src: url.to_string(),
            title,
            embed_url,
        }))
    }
}
