#![feature(try_trait, bool_to_option)]

extern crate serenity;
extern crate clap;
extern crate serde_json;

mod post_grab_api;
mod embed_bot;

use clap::Clap;

use serenity::Client;

use embed_bot::{EmbedBot, Settings};
use post_grab_api::*;
use std::fs::File;

#[derive(Clap)]
struct Opts {
    #[clap(short = "s", long = "settings-file")]
    settings_file: String,
}



fn main() {
    let opts: Opts = Opts::parse();

    let settings: Settings = File::open(&opts.settings_file)
        .map(|f| serde_json::from_reader(f).unwrap())
        .unwrap_or_default();

    let mut embedbot = EmbedBot::new(&opts.settings_file, settings);

    embedbot.register_api(reddit::RedditAPI::default());
    embedbot.register_api(ninegag::NineGagAPI::default());
    embedbot.register_api(imgur::ImgurAPI::default());

    let tok = std::env::var("DISCORD_TOKEN").expect("ENVVAR 'DISCORD_TOKEN' not found");
    let mut client = Client::new(&tok, embedbot).expect("could not create discord client");

    if let Err(e) = client.start() {
        eprintln!("[Error] Client Err: {:?}", e);
    }
}
