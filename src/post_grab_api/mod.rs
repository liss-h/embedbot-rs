pub mod imgur;
pub mod ninegag;
pub mod reddit;
pub mod svg;
pub mod util;

pub use util::*;

use crate::embed_bot::Settings;
use serenity::{
    async_trait,
    client::Context,
    model::{channel::Message, id::ChannelId, user::User},
};
use thiserror::Error;
use url::Url;

pub const USER_AGENT: &str = "embedbot v0.2";

#[derive(Debug, Error)]
pub enum Error {
    #[cfg(any(feature = "ninegag", feature = "reddit"))]
    #[error("invalid json")]
    JsonParse(#[from] serde_json::Error),

    #[cfg(any(feature = "ninegag", feature = "reddit"))]
    #[error("could not navigate to {0} in json")]
    JsonNav(&'static str),

    #[cfg(any(feature = "ninegag", feature = "reddit"))]
    #[error("expected {0}")]
    JsonConv(&'static str),

    #[error("HTTP GET failed")]
    HttpCommunication(#[from] reqwest::Error),

    #[error("expected url")]
    UrlParse(#[from] url::ParseError),

    #[cfg(feature = "svg")]
    #[error("invalid svg")]
    SvgParse(#[from] usvg::Error),
}

#[async_trait]
pub trait PostScraper {
    fn is_suitable(&self, url: &Url) -> bool;
    async fn get_post(&self, url: Url) -> Result<Box<dyn Post>, Error>;
}

#[async_trait]
pub trait Post: std::fmt::Debug + Send + Sync {
    fn should_embed(&self, settings: &Settings) -> bool;
    async fn send_embed(
        &self,
        u: &User,
        comment: Option<&str>,
        chan: ChannelId,
        ctx: &Context,
    ) -> Result<Message, Box<dyn std::error::Error>>;
}

#[macro_export]
#[cfg(any(feature = "reddit", feature = "ninegag"))]
macro_rules! nav_json {
    ($json:expr, $base_path:expr, $path:expr) => {
    	$json.and_then(|x| {
	    	x.get($path)
	    		.ok_or(Error::JsonNav(concat!($base_path, '.', $path)))
	    })
    };

    ($json:expr, $base_path:expr, $first_path:expr, $($path:expr),+) => {
        let _x = nav_json!{ $json, $base_path, $first_path };
        nav_json!{ _x, concat!($base_path, '.', $first_path), $($path),+ }
    };

    ($json:expr => $($path:expr)=>+) => {
    	{
    		nav_json!{ Ok(&$json), stringify!($json), $($path),+ }
    	}
    };

    ($json:expr => $($path:expr)=>+; as object) => {
    	{
    		let _x = {
    			nav_json!{ $json => $($path)=>+ }
    		};
    		_x.and_then(|x| x.as_object().ok_or(Error::JsonConv("Expected object")))
    	}
    };

    ($json:expr => $($path:expr)=>+; as array) => {
    	{
    		let _x = {
    			nav_json!{ $json => $($path)=>+ }
    		};
    		_x.and_then(|x| x.as_array().ok_or(Error::JsonConv("Expected array")))
    	}
    };

    ($json:expr => $($path:expr)=>+; as str) => {
    	{
    		let _x = {
    			nav_json!{ $json => $($path)=>+ }
    		};
    		_x.and_then(|x| x.as_str().ok_or(Error::JsonConv("Expected str")))
    	}
    };

    ($json:expr => $($path:expr)=>+; as bool) => {
    	{
    		let _x = {
    			nav_json!{ $json => $($path)=>+ }
    		};
    		_x.and_then(|x| x.as_bool().ok_or(Error::JsonConv("Expected bool")))
    	}
    };
}
