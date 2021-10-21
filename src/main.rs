#![feature(pattern, iter_intersperse)]

mod embed_bot;
mod post_grab_api;

use clap::Clap;
use embed_bot::{EmbedBot, Settings};
use serenity::Client;
use std::fs::File;

#[derive(Clap)]
struct Opts {
    #[clap(short = 's', long = "settings-file")]
    settings_file: String,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    let settings = match File::open(&opts.settings_file) {
        Ok(f) => {
            let s: Settings = serde_json::from_reader(f).unwrap();
            println!("Loaded Config: {:#?}", s);
            s
        }
        Err(e) => {
            let s = Settings::default();
            println!(
                "Unable to open config file (E: {:?}), using defaults: {:#?}",
                e, s
            );
            s
        }
    };

    let tok = std::env::var("DISCORD_TOKEN").expect("ENVVAR 'DISCORD_TOKEN' not found");

    let embed_bot = {
        let mut e = EmbedBot::new(&opts.settings_file);

        #[cfg(feature = "reddit")]
        e.register_api(post_grab_api::reddit::RedditAPI::default());

        #[cfg(feature = "ninegag")]
        e.register_api(post_grab_api::ninegag::NineGagAPI::default());

        #[cfg(feature = "imgur")]
        e.register_api(post_grab_api::imgur::ImgurAPI::default());

        #[cfg(feature = "svg")]
        e.register_api(post_grab_api::svg::SvgApi::default());

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
