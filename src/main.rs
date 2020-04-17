#![feature(try_trait, bool_to_option)]

extern crate serenity;

mod post_grab_api;
mod embed_bot;

use serenity::Client;

use embed_bot::EmbedBot;
use post_grab_api::*;

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
