use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::user::User;
use thiserror::Error;

use url::Url;
pub use util::*;

pub mod imgur;
pub mod ninegag;
pub mod reddit;
pub mod util;

pub const USER_AGENT: &str = "embedbot v0.2";

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid json")]
    JSONParseErr(#[from] serde_json::Error),

    #[error("could not navigate json")]
    JSONNavErr,

    #[error("HTTP GET failed")]
    HTTPErr(#[from] reqwest::Error),

    #[error("expected url")]
    UrlParserError(#[from] url::ParseError),
}

impl From<std::option::NoneError> for Error {
    fn from(_err: std::option::NoneError) -> Self {
        Error::JSONNavErr
    }
}

#[async_trait]
pub trait PostScraper {
    fn is_suitable(&self, url: &Url) -> bool;
    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error>;
}

pub trait Post: std::fmt::Debug + Send + Sync {
    fn should_embed(&self) -> bool;
    fn create_embed(&self, u: &User, comment: Option<&str>, create_message: &mut CreateMessage);
}
