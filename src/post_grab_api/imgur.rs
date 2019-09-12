use super::*;

use scraper::selector::Selector;

#[derive(Default)]
pub struct ImgurAPI;

impl PostGrabAPI for ImgurAPI {
    fn get_post(&mut self, url: &str) -> Result<Post, Error> {
        let html = wget_html(url, USER_AGENT)?;

        let title_selector = Selector::parse("title").unwrap();
        let img_selector = Selector::parse(r#"link[rel="image_src"]"#).unwrap();

        let title = {
            let tmp: String = html.select(&title_selector).next()?.text().collect();
            let beg = tmp.find(|ch: char| !ch.is_whitespace()).unwrap_or(0);

            tmp[beg..(tmp.len() - 8)].to_string()
        };

        let embed_url = html
            .select(&img_selector)
            .next()?
            .value()
            .attr("href")?
            .to_string();

        Ok(Post {
            website: "imgur".to_string(),
            origin: "imgur".to_string(),
            text: String::new(),
            title,
            embed_url,
            post_type: PostType::Image
        })
    }
}
