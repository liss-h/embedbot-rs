#![feature(iter_intersperse, pattern)]

mod embed_bot;
mod post_grab_api;

use embed_bot::{
    settings::{InitSettings, RuntimeSettings},
    EmbedBot, SettingsKey,
};
use serenity::{prelude::GatewayIntents, Client};
use std::fs::File;

pub const INIT_SETTINGS_PATH: &str = "/etc/embedbot/init.json";
pub const RUNTIME_SETTINGS_PATH: &str = "/etc/embedbot/runtime.json";

#[cfg(feature = "implicit-auto-embed")]
fn get_gateway_intents() -> GatewayIntents {
    GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT
}

#[cfg(not(feature = "implicit-auto-embed"))]
fn get_gateway_intents() -> GatewayIntents {
    GatewayIntents::empty()
}

#[tokio::main]
async fn main() {
    let init_settings = {
        let f = File::open(INIT_SETTINGS_PATH).expect("access to init settings file");
        let s: InitSettings = serde_json::from_reader(f).unwrap();
        println!("Loaded init settings: {:#?}", s);
        s
    };

    let runtime_settings = match File::open(RUNTIME_SETTINGS_PATH) {
        Ok(f) => {
            let s: RuntimeSettings = serde_json::from_reader(f).unwrap();
            println!("Loaded runtime settings: {:#?}", s);
            s
        }
        Err(e) => {
            let s = RuntimeSettings::default();
            println!("Unable to open runtime settings (E: {:?}), using defaults: {:#?}", e, s);
            s
        }
    };

    let embed_bot = {
        let mut e = EmbedBot::new(RUNTIME_SETTINGS_PATH);

        if let Some(modules) = init_settings.modules {
            #[cfg(feature = "reddit")]
            if let Some(settings) = modules.reddit {
                e.register_api(post_grab_api::reddit::Api { settings });
            }

            #[cfg(feature = "ninegag")]
            if let Some(settings) = modules.ninegag {
                e.register_api(post_grab_api::ninegag::Api { settings });
            }

            #[cfg(feature = "imgur")]
            if let Some(settings) = modules.imgur {
                e.register_api(post_grab_api::imgur::Api { settings });
            }

            #[cfg(feature = "svg")]
            if let Some(settings) = modules.svg {
                e.register_api(post_grab_api::svg::Api { settings });
            }
        }

        e
    };

    let mut client = Client::builder(
        &init_settings.discord_token,
        get_gateway_intents(),
    )
    .event_handler(embed_bot)
    .type_map_insert::<SettingsKey>(runtime_settings)
    .await
    .expect("could not create client");

    if let Err(e) = client.start().await {
        eprintln!("[Error] Client Err: {:?}", e);
    }
}
