use serenity::async_trait;
use serenity::builder::CreateMessage;
use serenity::model::user::User;
use thiserror::Error;

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
    JSONParseErr(#[from] serde_json::Error),

    #[error("could not navigate to {0} in json")]
    JSONNavErr(&'static str),

    #[error("expected {0}")]
    JsonConvError(&'static str),

    #[error("HTTP GET failed")]
    HTTPErr(#[from] reqwest::Error),

    #[error("expected url")]
    UrlParserError(#[from] url::ParseError),
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

#[macro_export]
macro_rules! nav_json {
    ($json:expr, $base_path:expr, $path:expr) => {
    	$json.and_then(|x| {
	    	x.get($path)
	    		.ok_or(Error::JSONNavErr(concat!($base_path, '.', $path)))
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
    		_x.and_then(|x| x.as_object().ok_or(Error::JsonConvError("Expected object")))
    	}
    };

    ($json:expr => $($path:expr)=>+; as array) => {
    	{
    		let _x = {
    			nav_json!{ $json => $($path)=>+ }
    		};
    		_x.and_then(|x| x.as_array().ok_or(Error::JsonConvError("Expected array")))
    	}
    };

    ($json:expr => $($path:expr)=>+; as str) => {
    	{
    		let _x = {
    			nav_json!{ $json => $($path)=>+ }
    		};
    		_x.and_then(|x| x.as_str().ok_or(Error::JsonConvError("Expected str")))
    	}
    };

    ($json:expr => $($path:expr)=>+; as bool) => {
    	{
    		let _x = {
    			nav_json!{ $json => $($path)=>+ }
    		};
    		_x.and_then(|x| x.as_bool().ok_or(Error::JsonConvError("Expected bool")))
    	}
    };
}
