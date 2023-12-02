#![feature(iter_intersperse, pattern)]

mod embed_bot;
mod post_grab_api;

use clap::Parser;
use embed_bot::{
    settings::{InitSettings, RuntimeSettings},
    EmbedBot, SettingsKey,
};
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
    #[clap(long, default_value = "/etc/embedbot/init.json")]
    init_conf: PathBuf,

    #[clap(long, default_value = "/etc/embedbot/runtime.json")]
    runtime_conf: PathBuf,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();

    let init_settings = {
        let f = File::open(opts.init_conf).expect("access to init settings file");
        let s: InitSettings = serde_json::from_reader(f).unwrap();
        tracing::info!("Loaded init settings: {:#?}", s);
        s
    };

    let runtime_settings = match File::open(&opts.runtime_conf) {
        Ok(f) => {
            let s: RuntimeSettings = serde_json::from_reader(f).unwrap();
            tracing::info!("Loaded runtime settings: {:#?}", s);
            s
        },
        Err(e) => {
            let s = RuntimeSettings::default();
            tracing::error!("Unable to open runtime settings (E: {:?}), using defaults: {:#?}", e, s);
            s
        },
    };

    let embed_bot = {
        let mut e = EmbedBot::new(&opts.runtime_conf);

        if let Some(modules) = init_settings.modules {
            #[cfg(feature = "reddit")]
            if let Some(settings) = modules.reddit {
                e.register_api(post_grab_api::reddit::Api { settings });
            }

            #[cfg(feature = "ninegag")]
            if let Some(settings) = modules.ninegag {
                e.register_api(post_grab_api::ninegag::Api { settings });
            }

            #[cfg(feature = "svg")]
            if let Some(settings) = modules.svg {
                e.register_api(post_grab_api::svg::Api { settings });
            }

            #[cfg(feature = "twitter")]
            if let Some(settings) = modules.twitter {
                e.register_api(post_grab_api::twitter::Api { settings });
            }
        }

        e
    };

    let mut client = Client::builder(&init_settings.discord_token, get_gateway_intents())
        .event_handler(embed_bot)
        .type_map_insert::<SettingsKey>(runtime_settings)
        .await
        .expect("could not create client");

    select! {
        res = client.start() => match res {
            Err(e) => tracing::error!("Client Err: {:?}", e),
            Ok(_) => {},
        },
        _ = tokio::signal::ctrl_c() => {}
    }
}
