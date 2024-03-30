#![cfg(feature = "svg")]

use super::{wget, CreateResponse, EmbedOptions, Error, Post as PostTrait, PostScraper, Url};
use resvg::{tiny_skia, usvg};
use serde::{Deserialize, Serialize};
use serenity::{async_trait, builder::CreateAttachment, model::user::User};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiSettings {}

pub struct Api {
    pub settings: ApiSettings,
}

#[derive(Debug)]
pub struct Post {
    src: Url,
    attachment: CreateAttachment,
}

impl Api {
    async fn scrape_post(url: Url) -> Result<Post, Error> {
        let res = wget(url.clone()).await?;
        let svg_str = res.text().await?;

        let svg = usvg::Tree::from_str(&svg_str, &usvg::Options::default(), &usvg::fontdb::Database::default())?;

        let size = svg.size();

        let mut pix = tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32).unwrap();
        resvg::render(&svg, usvg::Transform::identity(), &mut pix.as_mut());

        let path = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .unwrap()
            .into_temp_path();

        pix.save_png(&path).unwrap();

        let file = tokio::fs::File::create(path).await.unwrap();
        let name = url.path_segments().unwrap().rev().next().unwrap();
        let attachment = CreateAttachment::file(&file, name).await.unwrap();

        Ok(Post { src: url, attachment })
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
    fn create_embed<'data>(&'data self, u: &User, opts: &EmbedOptions, response: CreateResponse) -> CreateResponse {
        let discord_comment = opts
            .comment
            .as_ref()
            .map(|c| format!("**Comment By {author}:**\n{comment}\n\n", author = u.name, comment = c))
            .unwrap_or_default();

        response.add_file(self.attachment.clone()).content(format!(
            ">>> **{author}**\nSource: <{src}>\n\n{discord_comment}",
            author = u.name,
            src = &self.src,
            discord_comment = discord_comment,
        ))
    }
}
