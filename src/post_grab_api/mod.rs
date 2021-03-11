use async_trait::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::user::User;
use thiserror::Error;

pub use util::*;

pub mod ninegag;
pub mod reddit;
pub mod imgur;
pub mod util;

pub const USER_AGENT: &str = "embedbot v0.2";

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid json")]
    JSONParseErr(#[from] serde_json::Error),

    #[error("could not navigate json")]
    JSONNavErr,

    #[error("HTTP GET failed")]
    HTTPErr(#[from] reqwest::Error)
}

impl From<std::option::NoneError> for Error {
    fn from(_err: std::option::NoneError) -> Self {
        Error::JSONNavErr
    }
}


#[async_trait]
pub trait PostScraper {
    fn is_suitable(&self, url: &str) -> bool;
    async fn get_post(&self, url: &str) -> Result<Box<dyn Post>, Error>;
}


pub trait Post : std::fmt::Debug + Send + Sync {
    fn should_embed(&self) -> bool;
    fn create_embed(&self, u: &User, comment: Option<&str>, create_message: &mut CreateMessage);
}

