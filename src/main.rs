#![feature(try_trait, bind_by_move_pattern_guards)]

extern crate serenity;

use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

mod post_grab_api;
use post_grab_api::*;

const EMBED_CONTENT_MAX_LEN: usize = 2048;
const EMBED_TITLE_MAX_LEN: usize = 256;


fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn choose_grab_api(url: &str) -> Option<Box<dyn post_grab_api::PostGrabAPI>> {
    if url.starts_with("https://www.reddit.com") {
        Some(Box::new(post_grab_api::reddit::RedditAPI::default()))
    } else if url.starts_with("https://9gag.com") {
        Some(Box::new(post_grab_api::ninegag::NineGagAPI::default()))
    } else if url.starts_with("https://imgur.com/") {
        Some(Box::new(post_grab_api::imgur::ImgurAPI::default()))
    } else {
        None
    }
}

fn escape_markdown(title: &str) -> String {
    title
        .replace("*", "\\*")
        .replace("_", "\\_")
        .replace("~", "\\~")
        .replace(">", "\\>")
}

fn should_embed(post: &Post) -> bool {
    (&post.website == "9gag" && post.post_type == PostType::Video)
        || &post.website != "9gag"
}

fn limit_len(text: &str, limit: usize) -> String {
    const SHORTENED_MARKER: &str = " [...]";

    if text.len() > limit {
        format!("{}{}", &text[..(limit - SHORTENED_MARKER.len())], SHORTENED_MARKER)
    } else {
        text.to_string()
    }
}

fn fmt_title(post: &Post) -> String {
    let title = limit_len(
        &escape_markdown(&post.title),
        EMBED_TITLE_MAX_LEN - 9 - post.origin.len()); // -9 for formatting

    format!("'{}' - **{}**", title, post.origin)
}

fn default_embed<'a>(msg: &Message, post: &Post, e: &'a mut CreateEmbed) -> &'a mut CreateEmbed {
    e.title(&fmt_title(&post))
        .description(&limit_len(&post.text, EMBED_CONTENT_MAX_LEN))
        .author(|author_builder| author_builder.name(&msg.author.name))
        .url(&msg.content)
}

fn embed_image(msg: &Message, ctx: &Context, post: &Post) -> Result<Message, serenity::Error> {
    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| {
                default_embed(&msg, &post, e)
                    .image(&post.embed_url)
            })
        })
}

fn embed_video(msg: &Message, ctx: &Context, post: &Post) -> Result<Message, serenity::Error> {
    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.content(&format!(
                ">>> **{author}**\nSource: <{src}>\nEmbedURL: {embed_url}\n\n{title}\n\n{text}",
                author = msg.author.name,
                src = msg.content,
                embed_url = post.embed_url,
                title = fmt_title(&post),
                text = limit_len(&post.text, EMBED_CONTENT_MAX_LEN),
            ))
        })
}

fn embed_text(msg: &Message, ctx: &Context, post: &Post) -> Result<Message, serenity::Error> {
    msg.channel_id
        .send_message(&ctx.http, |m: &mut CreateMessage| {
            m.embed(|e: &mut CreateEmbed| default_embed(&msg, &post, e))
        })
}

fn send_embed_message(
    msg: &Message,
    ctx: &Context,
    post: &Post,
) -> Result<Message, serenity::Error> {
    match post.post_type {
        PostType::Image => embed_image(&msg, &ctx, &post),
        PostType::Video => match post.website.as_ref() {
                    "reddit" => embed_image(&msg, &ctx, &post), // embed thumbnail
                    _        => embed_video(&msg, &ctx, &post),
                },
        PostType::Text => embed_text(&msg, &ctx, &post)
    }
}

fn get_post(api: &mut dyn PostGrabAPI, url: &str) -> Result<Post, Error> {

    match api.get_post(url) {
        Ok(post) => Ok(post),
        Err(_) if url.ends_with("#") => get_post(api, url.trim_end_matches("#")),
        Err(e) => Err(e)
    }
}

struct EmbedBot;

impl EventHandler for EmbedBot {
    fn message(&self, context: Context, msg: Message) {
        if is_url(&msg.content) {
            match choose_grab_api(&msg.content) {
                Some(mut api) => match get_post(api.as_mut(), &msg.content) {
                    Ok(post) if should_embed(&post) => {
                        send_embed_message(&msg, &context, &post).expect("could not send msg");
                        msg.delete(context.http).expect("could not delete msg");

                        println!("[Info] embedded '{}' as {:?}", msg.content, post.post_type);
                    }
                    Ok(_) => println!(
                        "[Info] ignoring '{}'. Reason: not supposed to embed",
                        msg.content
                    ),
                    Err(e) => eprintln!("[Error] could not fetch post. Reason: {:?}", e),
                },
                None => println!(
                    "[Info] ignoring '{}'. Reason: no api available",
                    msg.content
                ),
            }
        }
    }

    fn ready(&self, _ctx: Context, _ready: Ready) {
        println!("[Info] logged in");
    }
}

fn main() {
    let tok = std::env::var("DISCORD_TOKEN").expect("ENVVAR 'DISCORD_TOKEN' not found");
    let mut client = Client::new(&tok, EmbedBot).expect("could not create client");

    if let Err(e) = client.start() {
        eprintln!("[Error] Client Err: {:?}", e);
    }
}
