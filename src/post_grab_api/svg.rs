#![cfg(feature = "svg")]

use super::{wget, Error, Post, PostScraper, Settings, Url, USER_AGENT};
use serenity::{
    async_trait,
    client::Context,
    model::{channel::Message, id::ChannelId, user::User},
};
use tempfile::NamedTempFile;
use tiny_skia::Pixmap;

#[derive(Copy, Clone, Default)]
pub struct SvgApi;

#[derive(Debug)]
pub struct SvgPost {
    src: Url,
    converted: NamedTempFile,
}

impl SvgApi {
    async fn scrape_post(url: Url) -> Result<SvgPost, Error> {
        let res = wget(url.clone(), USER_AGENT).await?;
        let svg_str = res.text().await?;

        let svg = usvg::Tree::from_str(&svg_str, &usvg::Options::default().to_ref())?;

        let size = svg.svg_node().size;

        let mut pix = Pixmap::new(size.width() as u32, size.height() as u32).unwrap();
        resvg::render(&svg, usvg::FitTo::Original, pix.as_mut()).unwrap();

        let path = tempfile::Builder::new().suffix(".png").tempfile().unwrap();

        pix.save_png(&path).unwrap();

        Ok(SvgPost {
            src: url,
            converted: path,
        })
    }
}

#[async_trait]
impl PostScraper for SvgApi {
    fn is_suitable(&self, url: &Url) -> bool {
        url.path()
            .trim_end_matches('/')
            .rsplit('.')
            .next()
            .map(|ext| ext.eq_ignore_ascii_case("svg"))
            == Some(true)
    }

    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error> {
        Ok(Box::new(Self::scrape_post(url).await?))
    }
}

#[async_trait]
impl Post for SvgPost {
    fn should_embed(&self, settings: &Settings) -> bool {
        settings.embed_settings.svg.0
    }

    async fn send_embed(
        &self,
        u: &User,
        comment: Option<&str>,
        _ignore_nsfw: bool,
        chan: ChannelId,
        ctx: &Context,
    ) -> Result<Message, Box<dyn std::error::Error>> {
        let msg = chan
            .send_files(ctx, [self.converted.path()], |m| {
                let discord_comment = comment
                    .map(|c| format!("**Comment By {author}:**\n{comment}\n\n", author = u.name, comment = c))
                    .unwrap_or_default();

                m.content(format!(
                    ">>> **{author}**\nSource: <{src}>\n\n{discord_comment}",
                    author = u.name,
                    src = &self.src,
                    discord_comment = discord_comment,
                ))
            })
            .await?;

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn svg_grab() {
        let url = "https://raw.githubusercontent.com/memononen/nanosvg/master/example/nano.svg"; // "https://upload.wikimedia.org/wikipedia/commons/0/09/Fedora_logo_and_wordmark.svg";
        let post = SvgApi::scrape_post(Url::from_str(url).unwrap()).await.unwrap();

        println!("{:?}", post.converted.keep().unwrap().1);
    }
}
