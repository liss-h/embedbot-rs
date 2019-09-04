#![feature(try_trait, bind_by_move_pattern_guards)]

extern crate serenity;

use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

mod post_grab_api;
use post_grab_api::*;

fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn choose_grab_api(url: &str) -> Option<Box<dyn post_grab_api::PostGrabAPI>> {
    if url.starts_with("https://www.reddit.com") {
        Some(Box::new(post_grab_api::reddit::RedditAPI::default()))
    } else if url.starts_with("https://9gag.com") {
        Some(Box::new(post_grab_api::ninegag::NineGagAPI::default()))
    } else {
        None
    }
}

fn escape_title(title: &str) -> String {
    title
        .replace("*", "\\*")
        .replace("_", "\\_")
        .replace("~", "\\~")
        .replace(">", "\\>")
}

fn should_embed(post: &Post) -> bool {
    (&post.website == "9gag" && post.post_type == PostType::Video)
        || (&post.website == "reddit" && post.post_type == PostType::Image)
}

fn send_embed_message(
    msg: &Message,
    ctx: &Context,
    post: &Post,
) -> Result<Message, serenity::Error> {
    match post.post_type {
        PostType::Image => msg
            .channel_id
            .send_message(&ctx.http, |m: &mut CreateMessage| {
                m.embed(|e: &mut CreateEmbed| {
                    e.title(&format!("'{}'", escape_title(&post.title)))
                        .description(&post.origin)
                        .author(|author_builder| author_builder.name(&msg.author.name))
                        .url(&msg.content)
                        .image(&post.embed_url)
                })
            }),
        PostType::Video => msg
            .channel_id
            .send_message(&ctx.http, |m: &mut CreateMessage| {
                m.content(&format!(
                    ">>> Sender: **{}**\nSource: <{}>\nEmbedURL: {}\n\n**\"{}\"**",
                    msg.author.name,
                    msg.content,
                    post.embed_url,
                    escape_title(&post.title)
                ))
            }),
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
