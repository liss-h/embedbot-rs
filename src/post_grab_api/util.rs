use super::Error;

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



pub const EMBED_CONTENT_MAX_LEN: usize = 2048;
pub const EMBED_TITLE_MAX_LEN: usize = 256;

pub fn escape_markdown(title: &str) -> String {
    title
        .replace("*", "\\*")
        .replace("_", "\\_")
        .replace("~", "\\~")
        .replace(">", "\\>")
}

pub fn limit_len(text: &str, limit: usize) -> String {
    const SHORTENED_MARKER: &str = " [...]";

    if text.len() > limit {
        format!("{}{}", &text[..(limit - SHORTENED_MARKER.len())], SHORTENED_MARKER)
    } else {
        text.to_string()
    }
}

pub fn limit_descr_len(text: &str) -> String {
    limit_len(text, EMBED_CONTENT_MAX_LEN)
}
