use reqwest::IntoUrl;
use serenity::{builder::CreateEmbed, model::user::User};
use std::borrow::Cow;
use url::Url;

const USER_AGENT: &str = concat!("github.com/Clueliss/embedbot-rs embedbot/", clap::crate_version!());
const EMBED_CONTENT_MAX_LEN: usize = 2048;

pub const EMBED_TITLE_MAX_LEN: usize = 256;

pub async fn wget<U: IntoUrl>(url: U) -> anyhow::Result<reqwest::Response> {
    let client = reqwest::Client::new();
    client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(Into::into)
}

pub async fn wget_json<U: IntoUrl>(url: U) -> anyhow::Result<serde_json::Value> {
    wget(url).await?.json().await.map_err(Into::into)
}

pub fn url_path_ends_with(haystack: &Url, needle: &str) -> bool {
    haystack.path().trim_end_matches('/').ends_with(needle)
}

pub fn url_path_ends_with_image_extension(haystack: &Url) -> bool {
    const EXTENSIONS: [&str; 11] = [
        ".jpg", ".png", ".gif", ".tif", ".bmp", ".dib", ".jpeg", ".jpe", ".jfif", ".tiff", ".heic",
    ];

    let s = haystack.path().trim_end_matches('/');

    EXTENSIONS.iter().any(|x| s.ends_with(x))
}

v_escape::new!(
    MarkdownEscape;
    '`' -> "\\`",
    '*' -> "\\*",
    '_' -> "\\_",
    '{' -> "\\{",
    '}' -> "\\}",
    '[' -> "\\[",
    ']' -> "\\]",
    '(' -> "\\(",
    ')' -> "\\)",
    '#' -> "\\#",
    '+' -> "\\+",
    '-' -> "\\-",
    '.' -> "\\.",
    '!' -> "\\!"
);

pub fn escape_markdown(title: &str) -> String {
    MarkdownEscape::new(title.as_bytes()).to_string()
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

pub fn include_author_comment(e: CreateEmbed, u: &User, comment: &str) -> CreateEmbed {
    let title = format!("Comment by {author}", author = u.name);
    e.field(title, comment, false)
}
