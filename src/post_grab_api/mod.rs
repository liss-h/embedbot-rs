pub mod ninegag;
pub mod reddit;
pub mod imgur;

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PostType {
    Image,
    Video,
    Text,
}

#[derive(Clone)]
pub struct Post {
    pub website: String,
    pub title: String,
    pub embed_url: String,
    pub origin: String,
    pub post_type: PostType,
    pub text: String,
    pub flair: String,
    pub nsfw: bool,
}

pub trait PostGrabAPI {
    fn is_suitable(&self, url: &str) -> bool;
    fn get_post(&self, url: &str) -> Result<Post, Error>;
}

pub fn wget(url: &str, user_agent: &str) -> Result<reqwest::Response, Error> {
    let client = reqwest::Client::new();
    client
        .get(&format!("{}/.json", url))
        .header("User-Agent", user_agent)
        .send()
        .map_err(|e| e.into())
}

pub fn wget_html(url: &str, user_agent: &str) -> Result<scraper::Html, Error> {
    let mut resp = wget(url, user_agent)?;
    Ok(scraper::Html::parse_document(&resp.text()?))
}

pub fn wget_json(url: &str, user_agent: &str) -> Result<serde_json::Value, Error> {
    let mut resp = wget(url, user_agent)?;
    resp.json().map_err(|e| e.into())
}
