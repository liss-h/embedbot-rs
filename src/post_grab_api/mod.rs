use serenity::async_trait;
use serenity::model::user::User;
use thiserror::Error;

use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::Context;
use url::Url;
pub use util::*;

pub mod imgur;
pub mod ninegag;
pub mod reddit;
pub mod svg;
pub mod util;

pub const USER_AGENT: &str = "embedbot v0.2";

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid json")]
    JsonParse(#[from] serde_json::Error),

    #[error("could not navigate to {0} in json")]
    JsonNav(&'static str),

    #[error("expected {0}")]
    JsonConv(&'static str),

    #[error("HTTP GET failed")]
    HttpCommunication(#[from] reqwest::Error),

    #[error("expected url")]
    UrlParse(#[from] url::ParseError),

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
    fn should_embed(&self) -> bool;
    async fn send_embed(
        &self,
        u: &User,
        comment: Option<&str>,
        chan: &ChannelId,
        ctx: &Context,
    ) -> Result<Message, Box<dyn std::error::Error>>;
}

#[macro_export]
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
