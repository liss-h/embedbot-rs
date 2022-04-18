use super::Error;
use reqwest::IntoUrl;
use serenity::{builder::CreateEmbed, model::user::User};
use std::{
    borrow::Cow,
    str::pattern::{Pattern, ReverseSearcher},
};
use url::Url;

pub async fn wget<U: IntoUrl>(url: U, user_agent: &str) -> Result<reqwest::Response, Error> {
    let client = reqwest::Client::new();
    client
        .get(url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(Into::into)
}

#[cfg(feature = "scraper")]
pub async fn wget_html<U: IntoUrl>(url: U, user_agent: &str) -> Result<scraper::Html, Error> {
    let resp = wget(url, user_agent).await?;
    Ok(scraper::Html::parse_document(&resp.text().await?))
}

pub async fn wget_json<U: IntoUrl>(url: U, user_agent: &str) -> Result<serde_json::Value, Error> {
    wget(url, user_agent).await?.json().await.map_err(Into::into)
}

pub fn url_path_ends_with<'a, P>(haystack: &'a Url, needle: P) -> bool
where
    P: Pattern<'a>,
    <P as Pattern<'a>>::Searcher: ReverseSearcher<'a>,
{
    haystack.path().trim_end_matches('/').ends_with(needle)
}

pub fn url_path_ends_with_image_extension(haystack: &Url) -> bool {
    const EXTENSIONS: [&str; 11] = [
        ".jpg", ".png", ".gif", ".tif", ".bmp", ".dib", ".jpeg", ".jpe", ".jfif", ".tiff", ".heic",
    ];

    let s = haystack.path().trim_end_matches('/');

    EXTENSIONS.iter().any(|x| s.ends_with(x))
}

pub const EMBED_CONTENT_MAX_LEN: usize = 2048;
pub const EMBED_TITLE_MAX_LEN: usize = 256;

pub fn escape_markdown(title: &str) -> String {
    const REPLACEMENTS: [(&str, &str); 14] = [
        ("`", "\\`"),
        ("*", "\\*"),
        ("_", "\\_"),
        ("{", "\\{"),
        ("}", "\\}"),
        ("[", "\\["),
        ("]", "\\]"),
        ("(", "\\("),
        (")", "\\)"),
        ("#", "\\#"),
        ("+", "\\+"),
        ("-", "\\-"),
        (".", "\\."),
        ("!", "\\!"),
    ];

    let mut buf = String::from(title);

    for (orig, new) in &REPLACEMENTS {
        buf = buf.replace(orig, new);
    }

    buf
}

pub fn limit_len(text: &str, limit: usize) -> Cow<str> {
    const SHORTENED_MARKER: &str = " [...]";

    if text.len() > limit {
        format!("{}{}", &text[..(limit - SHORTENED_MARKER.len())], SHORTENED_MARKER).into()
    } else {
        text.into()
    }
}

pub fn limit_descr_len(text: &str) -> Cow<'_, str> {
    limit_len(text, EMBED_CONTENT_MAX_LEN)
}

pub fn include_author_comment<'a>(e: &'a mut CreateEmbed, u: &User, comment: &str) -> &'a mut CreateEmbed {
    let title = format!("Comment by {author}", author = u.name);
    e.field(title, comment, false)
}
