use serenity::async_trait;
use tempfile::NamedTempFile;

use super::*;

#[derive(Copy, Clone)]
pub struct SvgApi;

#[derive(Debug)]
pub struct SvgPost(NamedTempFile);

#[async_trait]
impl PostScraper for SvgApi {
    fn is_suitable(&self, url: &Url) -> bool {
        url.path().trim_end_matches("/").ends_with(".svg")
    }

    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error> {
        let res = wget(url, USER_AGENT).await?;
        let svg_str = res.text().await?;
        let svg = nsvg::parse_str(&svg_str, nsvg::Units::Pixel, 96.0).unwrap();
        let img = svg.rasterize(1.0).unwrap();

        let path = NamedTempFile::new().unwrap();
        img.save(&path).unwrap();

        Ok(Box::new(SvgPost(path)))
    }
}

impl Post for SvgPost {
    fn should_embed(&self) -> bool {
        true
    }

    fn create_embed(&self, u: &User, comment: Option<&str>, create_message: &mut CreateMessage) {
        create_message.embed(|e| {
            e.author(|a| a.name(&u.name))
                .attachment(self.0.path().display());

            if let Some(comment) = comment {
                include_author_comment(e, u, comment);
            }

            e
        });
    }
}
