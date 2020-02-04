use serenity::builder::CreateMessage;
use serenity::model::user::User;

pub mod ninegag;
pub mod reddit;
pub mod imgur;
pub mod util;

pub use util::*;

pub const USER_AGENT: &str = "embedbot v0.1";

#[derive(Debug)]
pub enum Error {
    JSONParseErr(serde_json::Error),
    JSONNavErr,
    HTTPErr(reqwest::Error),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::HTTPErr(err)
    }
}

impl From<std::option::NoneError> for Error {
    fn from(_err: std::option::NoneError) -> Self {
        Error::JSONNavErr
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::JSONParseErr(err)
    }
}


pub trait PostGrabAPI {
    fn is_suitable(&self, url: &str) -> bool;
    fn get_post(&self, url: &str) -> Result<Box<dyn Post>, Error>;
}


pub trait Post {
    fn should_embed(&self) -> bool;
    fn create_embed(&self, u: &User, create_message: &mut CreateMessage);
}

