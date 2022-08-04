#![cfg(feature = "svg")]

use super::{wget, CreateResponse, EmbedOptions, Error, Post as PostTrait, PostScraper, Url};
use serde::{Deserialize, Serialize};
use serenity::{async_trait, model::user::User};
use tempfile::NamedTempFile;
use tiny_skia::Pixmap;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiSettings {}

pub struct Api {
    pub settings: ApiSettings,
}

#[derive(Debug)]
pub struct Post {
    src: Url,
    converted: NamedTempFile,
}

impl Api {
    async fn scrape_post(url: Url) -> Result<Post, Error> {
        let res = wget(url.clone()).await?;
        let svg_str = res.text().await?;

        let svg = usvg::Tree::from_str(&svg_str, &usvg::Options::default().to_ref())?;

        let size = svg.svg_node().size;

        let mut pix = Pixmap::new(size.width() as u32, size.height() as u32).unwrap();
        resvg::render(&svg, usvg::FitTo::Original, pix.as_mut()).unwrap();

        let path = tempfile::Builder::new().suffix(".png").tempfile().unwrap();

        pix.save_png(&path).unwrap();

        Ok(Post { src: url, converted: path })
    }
}

#[async_trait]
impl PostScraper for Api {
    type Output = Post;

    fn is_suitable(&self, url: &Url) -> bool {
        url.path()
            .trim_end_matches('/')
            .rsplit('.')
            .next()
            .map(|ext| ext.eq_ignore_ascii_case("svg"))
            == Some(true)
    }

    fn should_embed(&self, _post: &Self::Output) -> bool {
        true
    }

    async fn get_post(&self, url: Url) -> Result<Self::Output, Error> {
        Ok(Self::scrape_post(url).await?)
    }
}

impl PostTrait for Post {
    fn create_embed<'data>(&'data self, u: &User, opts: &EmbedOptions, response: CreateResponse<'_, 'data>) {
        let discord_comment = opts
            .comment
            .as_ref()
            .map(|c| format!("**Comment By {author}:**\n{comment}\n\n", author = u.name, comment = c))
            .unwrap_or_default();

        response.add_file(self.converted.path()).content(format!(
            ">>> **{author}**\nSource: <{src}>\n\n{discord_comment}",
            author = u.name,
            src = &self.src,
            discord_comment = discord_comment,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn svg_grab() {
        let url = "https://raw.githubusercontent.com/memononen/nanosvg/master/example/nano.svg"; // "https://upload.wikimedia.org/wikipedia/commons/0/09/Fedora_logo_and_wordmark.svg";
        let post = Api::scrape_post(Url::from_str(url).unwrap()).await.unwrap();

        println!("{:?}", post.converted.keep().unwrap().1);
    }
}
