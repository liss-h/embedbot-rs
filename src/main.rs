mod embed_bot;
mod post_grab_api;

use clap::Parser;
use embed_bot::{EmbedBot, Settings};
use serenity::{prelude::GatewayIntents, Client};
use std::{fs::File, path::PathBuf};
use tokio::select;

#[cfg(feature = "implicit-auto-embed")]
fn get_gateway_intents() -> GatewayIntents {
    GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT
}

#[cfg(not(feature = "implicit-auto-embed"))]
fn get_gateway_intents() -> GatewayIntents {
    GatewayIntents::empty()
}

#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "/etc/embedbot.json")]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();

    let settings = {
        let f = File::open(opts.config_path).expect("access to config file");
        let s: Settings = serde_json::from_reader(f).unwrap();
        tracing::info!("Loaded config: {:#?}", s);
        s
    };

    let embed_bot = {
        let mut e = EmbedBot::new();

        if let Some(modules) = settings.modules {
            #[cfg(feature = "reddit")]
            if let Some(settings) = modules.reddit {
                e.register_api(post_grab_api::reddit::Api::from_settings(settings));
            }

            #[cfg(feature = "ninegag")]
            if let Some(settings) = modules.ninegag {
                e.register_api(post_grab_api::ninegag::Api::from_settings(settings));
            }

            #[cfg(feature = "svg")]
            if let Some(settings) = modules.svg {
                e.register_api(post_grab_api::svg::Api::from_settings(settings));
            }

            #[cfg(feature = "twitter")]
            if let Some(settings) = modules.twitter {
                e.register_api(post_grab_api::twitter::Api::from_settings(settings));
            }
        }

        e
    };

    let mut client = Client::builder(&settings.discord_token, get_gateway_intents())
        .event_handler(embed_bot)
        .await
        .expect("could not create client");

    select! {
        res = client.start() => if let Err(e) = res {
            tracing::error!("Client error: {:?}", e);
        },
        _ = tokio::signal::ctrl_c() => {
        },
    }
}
