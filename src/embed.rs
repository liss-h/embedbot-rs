use crate::post_grab_api::Post;

use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::channel::Message;

const EMBED_CONTENT_MAX_LEN: usize = 2048;
const EMBED_TITLE_MAX_LEN: usize = 256;

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

pub fn fmt_title(post: &Post) -> String {
    let title = limit_len(
        &escape_markdown(&post.title),
        EMBED_TITLE_MAX_LEN - 9 - post.origin.len()); // -9 for formatting

    if post.flair.is_empty() {
        format!("'{}' - **{}**", title, post.origin)
    } else {
        format!("'{}' [{}] - **{}**", title, post.flair, post.origin)
    }
}

pub fn default_embed<'a>(e: &'a mut CreateEmbed, msg: &Message, post: &Post) -> &'a mut CreateEmbed {
    e.title(&fmt_title(&post))
        .description(&limit_len(&post.text, EMBED_CONTENT_MAX_LEN))
        .author(|author_builder| author_builder.name(&msg.author.name))
        .url(&msg.content)
}

pub fn image_embed(m: &mut CreateMessage, msg: &Message, post: &Post) {
    m.embed(|e| default_embed(e, msg, post).image(&post.embed_url));
}

pub fn video_thumbnail_embed(m: &mut CreateMessage, msg: &Message, post: &Post) {
    m.embed(|e| {
        e.title(&fmt_title(&post))
            .description("[click title to watch video]")
            .author(|a| a.name(&msg.author.name))
            .url(&msg.content)
            .image(&post.embed_url)
    });
}

pub fn video_embed(m: &mut CreateMessage, msg: &Message, post: &Post) {
    m.content(format!(
        ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{title}\n\n{text}",
        author = msg.author.name,
        src = msg.content,
        embed_url = post.embed_url,
        title = fmt_title(&post),
        text = limit_len(&post.text, EMBED_CONTENT_MAX_LEN),
    ));
}

pub fn text_embed(m: &mut CreateMessage, msg: &Message, post: &Post) {
    m.embed(|e| default_embed(e, msg, post));
}
