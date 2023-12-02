pub mod settings;

use crate::post_grab_api::{CreateResponse, DynPostScraper, EmbedOptions, Error, Post};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{
        application::{
            command::{Command, CommandOptionType, CommandType},
            interaction::{
                application_command::{CommandData, CommandDataOption},
                Interaction,
            },
        },
        channel::Message,
        gateway::Ready,
    },
    prelude::TypeMapKey,
};
use settings::RuntimeSettings;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use url::Url;

pub struct SettingsKey;

impl TypeMapKey for SettingsKey {
    type Value = RuntimeSettings;
}

#[derive(Default)]
pub struct EmbedBot {
    settings_path: PathBuf,
    apis: Vec<Box<dyn DynPostScraper + Send + Sync>>,
}

impl EmbedBot {
    pub fn new<P: AsRef<Path>>(settings_path: P) -> Self {
        EmbedBot { settings_path: settings_path.as_ref().to_path_buf(), apis: Vec::new() }
    }

    pub fn find_api(&self, url: &Url) -> Option<&(dyn DynPostScraper + Send + Sync)> {
        self.apis.iter().find(|a| a.is_suitable(url)).map(AsRef::as_ref)
    }

    pub fn register_api<T: 'static + DynPostScraper + Send + Sync>(&mut self, api: T) {
        self.apis.push(Box::new(api));
    }

    async fn get_post(&self, mut url: Url) -> Result<Box<dyn Post>, Error> {
        if let Some(api) = self.find_api(&url) {
            url.set_fragment(None);
            api.get_dyn_post(url).await
        } else {
            Err(Error::NoApiAvailable)
        }
    }

    fn reply_error(msg: &str, response: CreateResponse) {
        response.embed(|e| e.title(":x: Error").description(msg));
    }
}

#[async_trait]
impl EventHandler for EmbedBot {
    #[cfg(feature = "implicit-auto-embed")]
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            let do_implicit_auto_embed = ctx
                .data
                .read()
                .await
                .get::<SettingsKey>()
                .unwrap()
                .do_implicit_auto_embed;

            if do_implicit_auto_embed {
                let content: Vec<_> = msg.content.lines().collect();

                let (url, comment) = match &content[..] {
                    [] => (None, None),
                    [a] => (Url::parse(a).ok(), None),
                    args => {
                        let (urls, comments): (Vec<_>, Vec<_>) = args
                            .iter()
                            .filter(|s| !s.is_empty())
                            .partition(|a| Url::parse(a).is_ok());

                        let mut urls = urls.into_iter().map(|u| Url::parse(u).unwrap());

                        let comments: String = comments.into_iter().intersperse("\n").collect();

                        (urls.next(), Some(comments))
                    },
                };

                if let Some(url) = url {
                    match self.get_post(url.clone()).await {
                        Ok(post) => {
                            msg.channel_id
                                .send_message(&ctx, |response| {
                                    post.create_embed(
                                        &msg.author,
                                        &EmbedOptions { comment, ..Default::default() },
                                        CreateResponse::Message(response),
                                    );
                                    response
                                })
                                .await
                                .unwrap();

                            msg.delete(&ctx).await.unwrap();
                        },
                        Err(Error::NoApiAvailable) => {
                            tracing::info!("not embedding {}: no api available", url);
                        },
                        Err(Error::NotSupposedToEmbed(_)) => {
                            tracing::info!("ignoring {}: not supposed to embed", url);
                        },
                        Err(e) => {
                            tracing::error!("[Error] while trying to embed {}: {}", url, e);
                        },
                    }
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, _ready: Ready) {
        Command::create_global_application_command(&ctx, |command| {
            command
                .name("embed")
                .kind(CommandType::ChatInput)
                .description("embed a post")
                .create_option(|option| {
                    option
                        .name("url")
                        .description("url of the post")
                        .required(true)
                        .kind(CommandOptionType::String)
                })
                .create_option(|option| {
                    option
                        .name("ignore-nsfw")
                        .description("embed fully even if post is flagged as nsfw")
                        .required(false)
                        .kind(CommandOptionType::Boolean)
                })
                .create_option(|option| {
                    option
                        .name("ignore-spoiler")
                        .description("embed fully even if post is flagged as spoiler")
                        .required(false)
                        .kind(CommandOptionType::Boolean)
                })
                .create_option(|option| {
                    option
                        .name("comment")
                        .description("a personal comment to include")
                        .required(false)
                        .kind(CommandOptionType::String)
                })
        })
        .await
        .unwrap();

        Command::create_global_application_command(&ctx, |command| {
            command
                .name("settings")
                .description("view or modify bot settings")
                .kind(CommandType::ChatInput)
                .create_option(|option| {
                    option
                        .name("get")
                        .description("view a bot setting")
                        .kind(CommandOptionType::SubCommandGroup);

                    #[cfg(feature = "implicit-auto-embed")]
                    option.create_sub_option(|option| {
                        option
                            .name("do-implicit-auto-embed")
                            .description("try to embed urls even when not explicitly called")
                            .kind(CommandOptionType::SubCommand)
                    });

                    option
                })
                .create_option(|option| {
                    option
                        .name("set")
                        .description("change a bot setting")
                        .kind(CommandOptionType::SubCommandGroup);

                    #[cfg(feature = "implicit-auto-embed")]
                    option.create_sub_option(|option| {
                        option
                            .name("do-implicit-auto-embed")
                            .description("try to embed urls even when not explicitly called")
                            .kind(CommandOptionType::SubCommand)
                            .create_sub_option(|option| {
                                option
                                    .name("value")
                                    .description("the new value")
                                    .required(true)
                                    .kind(CommandOptionType::Boolean)
                            })
                    });

                    option
                })
        })
        .await
        .unwrap();

        tracing::info!("logged in");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = &interaction {
            match &command.data {
                CommandData { name, options, .. } if name == "embed" => {
                    let url = options
                        .iter()
                        .find(|c| c.name == "url")
                        .unwrap()
                        .value
                        .as_ref()
                        .unwrap()
                        .as_str()
                        .unwrap();

                    let comment = options
                        .iter()
                        .find(|c| c.name == "comment")
                        .and_then(|c| c.value.as_ref())
                        .map(|c| c.as_str().unwrap().to_owned());

                    let ignore_nsfw = options
                        .iter()
                        .find(|c| c.name == "ignore-nsfw")
                        .and_then(|c| c.value.as_ref())
                        .map(|c| c.as_bool().unwrap())
                        .unwrap_or(false);

                    let ignore_spoiler = options
                        .iter()
                        .find(|c| c.name == "ignore-spoiler")
                        .and_then(|c| c.value.as_ref())
                        .map(|c| c.as_bool().unwrap())
                        .unwrap_or(false);

                    let opts = EmbedOptions { comment, ignore_nsfw, ignore_spoiler };

                    match Url::parse(url) {
                        Ok(url) => {
                            let user = &command.user;

                            match self.get_post(url.clone()).await {
                                Ok(post) => {
                                    command
                                        .create_interaction_response(&ctx, |resp| {
                                            resp.interaction_response_data(|data| {
                                                post.create_embed(user, &opts, CreateResponse::Interaction(data));
                                                data
                                            })
                                        })
                                        .await
                                        .unwrap();

                                    tracing::trace!("embedded '{}': {:?}", url, post);
                                },
                                Err(e) => {
                                    let msg = format!("{}", e);
                                    tracing::error!("[Error] {msg}");

                                    command
                                        .create_interaction_response(&ctx, |resp| {
                                            resp.interaction_response_data(|data| {
                                                Self::reply_error(&msg, CreateResponse::Interaction(data));
                                                data
                                            })
                                        })
                                        .await
                                        .unwrap();
                                },
                            }
                        },
                        Err(_) => {
                            command
                                .create_interaction_response(&ctx, |resp| {
                                    resp.interaction_response_data(|data| {
                                        Self::reply_error(
                                            &format!("Could not parse url: {}", url),
                                            CreateResponse::Interaction(data),
                                        );
                                        data
                                    })
                                })
                                .await
                                .unwrap();
                        },
                    }
                },
                CommandData { name, options, .. } if name == "settings" => {
                    let reply_invalid_setting = command.create_interaction_response(&ctx, |response| {
                        response.interaction_response_data(|data| {
                            Self::reply_error("invalid setting", CreateResponse::Interaction(data));
                            data
                        })
                    });

                    match options.first().unwrap() {
                        CommandDataOption { name, options, .. } if name == "get" => {
                            let key = &options.first().unwrap().name;

                            let data = ctx.data.read().await;
                            let settings = data.get::<SettingsKey>().unwrap();

                            let value: String = match key.as_str() {
                                #[cfg(feature = "implicit-auto-embed")]
                                "do-implicit-auto-embed" => settings.do_implicit_auto_embed.to_string(),
                                _ => {
                                    reply_invalid_setting.await.unwrap();

                                    return;
                                },
                            };

                            command
                                .create_interaction_response(&ctx, |response| {
                                    response.interaction_response_data(|data| {
                                        data.embed(|e| {
                                            e.title(":ballot_box_with_check: Current setting value")
                                                .field(key, value, true)
                                        })
                                    })
                                })
                                .await
                                .unwrap();
                        },
                        CommandDataOption { name, options, .. } if name == "set" => {
                            let key_opt = &options.first().unwrap();

                            let key = &key_opt.name;

                            let mut data = ctx.data.write().await;
                            let settings = data.get_mut::<SettingsKey>().unwrap();

                            #[allow(unused)]
                            let value = &key_opt
                                .options
                                .iter()
                                .find(|c| c.name == "value")
                                .unwrap()
                                .value
                                .as_ref()
                                .unwrap();

                            match key.as_str() {
                                #[cfg(feature = "implicit-auto-embed")]
                                "do-implicit-auto-embed" => {
                                    settings.do_implicit_auto_embed = value.as_bool().unwrap();
                                },
                                _ => {
                                    reply_invalid_setting.await.unwrap();
                                    return;
                                },
                            }

                            match File::create(&self.settings_path) {
                                Ok(f) => {
                                    serde_json::to_writer_pretty(f, settings).unwrap();
                                },
                                Err(e) => {
                                    tracing::error!("unable to persist runtime settings: {}", e);
                                },
                            }

                            command
                                .create_interaction_response(&ctx, |response| {
                                    response.interaction_response_data(|data| {
                                        data.embed(|e| {
                                            e.title(":ballot_box_with_check: Changed setting value")
                                                .field(key, value, true)
                                        })
                                    })
                                })
                                .await
                                .unwrap();
                        },
                        _ => panic!("invalid settings subcommand received"),
                    }
                },
                _ => (),
            }
        }
    }
}
