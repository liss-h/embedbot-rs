use std::fmt::Display;
use std::fs::File;
use std::path::{Path, PathBuf};

use clap::Clap;
use serde::{Deserialize, Serialize};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::model::user::User;
use serenity::prelude::*;
use strum::AsStaticRef;

use crate::embed_bot::interface::*;

use super::post_grab_api::*;

pub mod interface;

pub fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub prefix: String,
    pub do_implicit_auto_embed: bool,
}

impl Settings {
    pub fn display_value(&self, opt: SettingsOptions) -> &dyn Display {
        match opt {
            SettingsOptions::Prefix => &self.prefix,
            SettingsOptions::DoImplicitAutoEmbed => &self.do_implicit_auto_embed,
        }
    }
}

impl TypeMapKey for Settings {
    type Value = Settings;
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            prefix: "~".to_string(),
            do_implicit_auto_embed: true,
        }
    }
}

#[derive(Default)]
pub struct EmbedBot {
    settings_path: PathBuf,
    apis: Vec<Box<dyn PostScraper + Send + Sync>>,
}

impl EmbedBot {
    pub fn new<P: AsRef<Path>>(settings_path: P) -> Self {
        EmbedBot {
            settings_path: settings_path.as_ref().to_path_buf(),
            apis: Vec::new(),
        }
    }

    pub fn find_api(&self, url: &str) -> Option<&(dyn PostScraper + Send + Sync)> {
        self.apis
            .iter()
            .find(|a| a.is_suitable(url))
            .map(|a| a.as_ref())
    }

    pub fn register_api<T: 'static + PostScraper + Send + Sync>(&mut self, api: T) {
        self.apis.push(Box::new(api));
    }

    async fn embed(
        &self,
        ctx: &Context,
        chan: ChannelId,
        author: &User,
        url: &str,
        comment: Option<&str>,
    ) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        match self.find_api(&url) {
            Some(api) => match api.get_post(url.trim_end_matches('#')).await {
                Ok(post) if post.should_embed() => {
                    let msg = chan
                        .send_message(ctx, |m| {
                            post.create_embed(author, comment.as_deref(), m);
                            m
                        })
                        .await?;

                    println!("[Info] embedded '{}': {:?}", url, post);
                    Ok(Some(msg))
                }
                Ok(_) => {
                    println!("[Info] ignoring '{}'. Reason: not supposed to embed", url);
                    Ok(None)
                }
                Err(e) => {
                    eprintln!("[Error] could not fetch post. Reason: {:?}", e);
                    Err(e.into())
                }
            },
            None => {
                println!("[Info] ignoring '{}'. Reason: no api available", url);
                Ok(None)
            }
        }
    }

    async fn reply_success(chan: ChannelId, ctx: &Context, msg: &str) -> serenity::Result<Message> {
        chan.send_message(ctx, |m| {
            m.content(format!(":white_check_mark: Success\n{}", msg))
        })
        .await
    }

    async fn reply_error(chan: ChannelId, ctx: &Context, msg: &str) -> serenity::Result<Message> {
        chan.send_message(ctx, |m| m.content(format!(":x: Error\n{}", msg)))
            .await
    }
}

#[async_trait]
impl EventHandler for EmbedBot {
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
            let mut data = ctx.data.write().await;
            let mut settings = data.get_mut::<Settings>().unwrap();

            if msg.content.starts_with(&settings.prefix) {
                let opts = EmbedBotOpts::try_parse_from(command_line_split(
                    &msg.content.trim_start_matches(&settings.prefix),
                ));

                match opts {
                    Ok(EmbedBotOpts::Embed { url, comment }) => {
                        self.embed(&ctx, msg.channel_id, &msg.author, &url, comment.as_deref())
                            .await
                            .unwrap();
                    }
                    Ok(EmbedBotOpts::Settings(SettingsSubcommand::Get { key })) => {
                        Self::reply_success(
                            msg.channel_id,
                            &ctx,
                            &format!(
                                "```c\n{key} == {value}\n```",
                                key = key.as_static(),
                                value = settings.display_value(key)
                            ),
                        )
                        .await
                        .unwrap();
                    }
                    Ok(EmbedBotOpts::Settings(SettingsSubcommand::Set { key, value })) => {
                        let res = match key {
                            SettingsOptions::Prefix => {
                                settings.prefix = value.clone();
                                Ok(())
                            }
                            SettingsOptions::DoImplicitAutoEmbed => {
                                if let Ok(value) = value.parse::<bool>() {
                                    settings.do_implicit_auto_embed = value;
                                    Ok(())
                                } else {
                                    Err("expected boolean".to_owned())
                                }
                            }
                        };

                        match res {
                            Ok(()) => {
                                let m = format!(
                                    "```c\n{key} := {value}\n```",
                                    key = key.as_static(),
                                    value = value
                                );

                                Self::reply_success(msg.channel_id, &ctx, &m).await.unwrap();

                                if let Ok(f) = File::create(&self.settings_path) {
                                    serde_json::to_writer(f, &settings).unwrap();
                                }
                            }
                            Err(e) => {
                                Self::reply_error(msg.channel_id, &ctx, &format!("```{}```", e))
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        Self::reply_error(msg.channel_id, &ctx, &format!("```{}```", e))
                            .await
                            .unwrap();
                    }
                }
            } else if settings.do_implicit_auto_embed {
                let content: Vec<_> = msg.content.lines().collect();

                let (url, comment) = match &content[..] {
                    [] => (None, None),
                    [a] => (is_url(a).then(|| a.to_string()), None),
                    args => {
                        let (urls, comments): (Vec<_>, Vec<_>) = args
                            .iter()
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .partition(|a| is_url(a));

                        (urls.into_iter().next(), Some(comments.join("\n")))
                    }
                };

                if let Some(url) = url {
                    let reply = self
                        .embed(&ctx, msg.channel_id, &msg.author, &url, comment.as_deref())
                        .await
                        .unwrap();

                    if reply.is_some() {
                        msg.delete(&ctx).await.unwrap();
                    }
                }
            }
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {
        println!("[Info] logged in");
    }
}
