#![feature(try_trait, bind_by_move_pattern_guards)]

extern crate serenity;

use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

mod post_grab_api;
use post_grab_api::*;

mod embed;
use embed::*;


fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn should_embed(post: &Post) -> bool {
    (&post.website == "9gag" && post.post_type == PostType::Video)
        || &post.website != "9gag"
}

fn send_embed_message(
    ctx: &Context,
    msg: &Message,
    post: &Post,
) -> Result<Message, serenity::Error> {
    msg.channel_id.send_message(ctx, |m| {
        match post.post_type {
            PostType::Image => image_embed(m, msg, post),
            PostType::Text  => text_embed(m, msg, post),
            PostType::Video if &post.website == "reddit" => video_thumbnail_embed(m, msg, post),
            PostType::Video => video_embed(m, msg, post),
        }

        m
    })
}

fn get_post(api: &dyn PostGrabAPI, url: &str) -> Result<Post, Error> {

    match api.get_post(url) {
        Ok(post) => Ok(post),
        Err(_) if url.ends_with("#") => get_post(api, url.trim_end_matches("#")),
        Err(e) => Err(e)
    }
}

#[derive(Default)]
struct EmbedBot {
    apis: Vec<Box<dyn PostGrabAPI + Send + Sync>>
}

impl EmbedBot {
    fn new() -> Self {
        Self::default()
    }

    fn find_api(&self, url: &str) -> Option<&dyn PostGrabAPI> {
        self.apis
            .iter()
            .find(|a| a.is_suitable(url))
            .map(|a| a.as_ref() as &dyn PostGrabAPI)
    }

    fn register_api<T: 'static + PostGrabAPI + Send + Sync>(&mut self, api: T) {
        self.apis.push(Box::new(api));
    }
}

impl EventHandler for EmbedBot {
    fn message(&self, context: Context, msg: Message) {
        if is_url(&msg.content) {

            match self.find_api(&msg.content) {
                Some(api) => match get_post(api, &msg.content) {
                    Ok(post) if should_embed(&post) => {
                        send_embed_message(&context, &msg, &post).expect("could not send msg");
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
    let mut embedbot = EmbedBot::new();

    embedbot.register_api(reddit::RedditAPI::default());
    embedbot.register_api(ninegag::NineGagAPI::default());
    embedbot.register_api(imgur::ImgurAPI::default());

    let tok = std::env::var("DISCORD_TOKEN").expect("ENVVAR 'DISCORD_TOKEN' not found");
    let mut client = Client::new(&tok, embedbot).expect("could not create discord client");

    if let Err(e) = client.start() {
        eprintln!("[Error] Client Err: {:?}", e);
    }
}
