#![feature(try_trait, bool_to_option)]

extern crate clap;
extern crate reqwest;
extern crate scraper;
extern crate serde;
extern crate serde_json;
extern crate serenity;
extern crate strum;
extern crate thiserror;
extern crate tokio;

use std::fs::File;

use clap::Clap;
use serenity::Client;

use post_grab_api::*;

use crate::embed_bot::{EmbedBot, Settings};

mod embed_bot;
mod post_grab_api;

#[derive(Clap)]
struct Opts {
    #[clap(short = 's', long = "settings-file")]
    settings_file: String,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    let settings: Settings = File::open(&opts.settings_file)
        .map(|f| serde_json::from_reader(f).unwrap())
        .unwrap_or_default();

    let tok = std::env::var("DISCORD_TOKEN").expect("ENVVAR 'DISCORD_TOKEN' not found");

    let embed_bot = {
        let mut e = EmbedBot::new(&opts.settings_file);
        e.register_api(reddit::RedditAPI::default());
        e.register_api(ninegag::NineGagAPI::default());
        e.register_api(imgur::ImgurAPI::default());

        e
    };

    let mut client = Client::builder(&tok)
        .event_handler(embed_bot)
        .type_map_insert::<Settings>(settings)
        .await
        .expect("could not create client");

    if let Err(e) = client.start().await {
        eprintln!("[Error] Client Err: {:?}", e);
    }
}
