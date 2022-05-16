pub mod ninegag;
pub mod reddit;
pub mod svg;
pub mod util;

use serenity::{
    async_trait,
    builder::{CreateEmbed, CreateInteractionResponseData, CreateMessage},
    model::{channel::AttachmentType, user::User},
};
use thiserror::Error;
use url::Url;
pub use util::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error("not supposed to embed")]
    NotSupposedToEmbed(Box<dyn Post>),

    #[error("no api available")]
    NoApiAvailable,

    #[cfg(any(feature = "ninegag", feature = "reddit"))]
    #[error("invalid json")]
    JsonParse(#[from] serde_json::Error),

    #[cfg(any(feature = "ninegag", feature = "reddit"))]
    #[error("navigation error: {0}")]
    JsonNav(#[from] json_nav::JsonNavError),

    #[error("HTTP GET failed")]
    HttpCommunication(#[from] reqwest::Error),

    #[error("expected url")]
    UrlParse(#[from] url::ParseError),

    #[cfg(feature = "svg")]
    #[error("invalid svg")]
    SvgParse(#[from] usvg::Error),

    #[cfg(any(feature = "imgur", feature = "ninegag"))]
    #[error("general navigation error: {0}")]
    Navigation(String),
}

#[derive(Debug, Default)]
pub struct EmbedOptions {
    pub comment: Option<String>,
    pub ignore_nsfw: bool,
    pub ignore_spoiler: bool,
}

pub enum CreateResponse<'r, 'data> {
    #[cfg(feature = "implicit-auto-embed")]
    Message(&'r mut CreateMessage<'data>),
    Interaction(&'r mut CreateInteractionResponseData<'data>),
}

impl<'a> CreateResponse<'_, 'a> {
    pub fn content<S: ToString>(self, s: S) -> Self {
        match self {
            #[cfg(feature = "implicit-auto-embed")]
            CreateResponse::Message(response) => CreateResponse::Message(response.content(s)),
            CreateResponse::Interaction(response) => CreateResponse::Interaction(response.content(s)),
        }
    }

    pub fn embed<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut CreateEmbed) -> &mut CreateEmbed,
    {
        match self {
            #[cfg(feature = "implicit-auto-embed")]
            CreateResponse::Message(response) => CreateResponse::Message(response.embed(f)),
            CreateResponse::Interaction(response) => CreateResponse::Interaction(response.embed(f)),
        }
    }

    pub fn add_file<T: Into<AttachmentType<'a>>>(self, file: T) -> Self {
        match self {
            #[cfg(feature = "implicit-auto-embed")]
            CreateResponse::Message(response) => CreateResponse::Message(response.add_file(file)),
            CreateResponse::Interaction(response) => CreateResponse::Interaction(response.add_file(file)),
        }
    }
}

#[async_trait]
pub trait PostScraper {
    type Output: Post;

    fn is_suitable(&self, url: &Url) -> bool;
    fn should_embed(&self, post: &Self::Output) -> bool;

    async fn get_post(&self, url: Url) -> Result<Self::Output, Error>;
}

pub trait Post: std::fmt::Debug + Send + Sync {
    fn create_embed<'data>(&'data self, u: &User, opts: &EmbedOptions, response: CreateResponse<'_, 'data>);
}

#[async_trait]
pub trait DynPostScraper {
    fn is_suitable(&self, url: &Url) -> bool;
    async fn get_dyn_post(&self, url: Url) -> Result<Box<dyn Post>, Error>;
}

#[async_trait]
impl<PS, O> DynPostScraper for PS
where
    PS: PostScraper<Output = O> + Sync,
    O: Post + 'static,
{
    fn is_suitable(&self, url: &Url) -> bool {
        PostScraper::is_suitable(self, url)
    }

    async fn get_dyn_post(&self, url: Url) -> Result<Box<dyn Post>, Error> {
        let p = self.get_post(url).await?;

        if self.should_embed(&p) {
            Ok(Box::new(p))
        } else {
            Err(Error::NotSupposedToEmbed(Box::new(p)))
        }
    }
}
