pub mod ninegag;
pub mod reddit;

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

#[derive(PartialEq, Eq)]
pub enum PostType {
    Image,
    Video,
}

pub struct Post {
    pub website: String,
    pub title: String,
    pub embed_url: String,
    pub origin: String,
    pub post_type: PostType,
}

pub trait PostGrabAPI {
    fn get_post(&mut self, url: &str) -> Result<Post, Error>;
}
