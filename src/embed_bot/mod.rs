use std::fs::File;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::*;

use super::post_grab_api::*;

pub mod interface;

const SETTINGS_CHOICES: [&str; 2] = ["prefix", "do-implicit-auto-embed"];
const SETTINGS_CHOICES_DESCR: [&str; 2] = [":exclamation: prefix", ":envelope: do-implicit-auto-embed"];


pub fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub prefix: String,
    pub do_implicit_auto_embed: bool,
}

impl TypeMapKey for Settings {
    type Value = Settings;
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            prefix: "*".to_string(),
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

    async fn embed_subroutine(&self, ctx: &Context, msg: &Message, url: &str, comment: Option<&str>) {
        match self.find_api(&url) {
            Some(api) => match api.get_post(url.trim_end_matches('#')).await {
                Ok(post) if post.should_embed() => {
                    msg.channel_id.send_message(ctx, |m| {
                        post.create_embed(&msg.author, comment, m);
                        m
                    })
                    .await
                    .expect("could not send msg");

                    msg.delete(ctx)
                        .await
                        .expect("could not delete msg");

                    println!("[Info] embedded '{}': {:?}", url, post);
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

    async fn reply_success(chan: ChannelId, ctx: &Context, mes: &str) -> serenity::Result<Message> {
        chan
            .send_message(&ctx, |mb| mb.content(format!(":white_check_mark: Success: {}", mes)))
            .await
    }

    async fn reply_error(chan: ChannelId, ctx: &Context, mes: &str) -> serenity::Result<Message> {
        chan
            .send_message(&ctx, |mb| mb.content(format!(":x: Error: {}", mes)))
            .await
    }

    async fn settings_subroutine(&self, settings: &mut Settings, ctx: &Context, msg: &Message, args: &[&str]) {

        if args.is_empty() {
            msg.channel_id
                .send_message(&ctx, |m| {
                    m.embed(|e| {

                        e.title("EmbedBot Settings")
                            .description(format!("Use the command format `{}settings <option>`", settings.prefix));

                        for (choice, descr) in SETTINGS_CHOICES.iter().zip(SETTINGS_CHOICES_DESCR.iter()) {
                            e.field(
                                descr,
                                format!("`{}settings {}`", settings.prefix, choice), true);
                        }

                        e
                    })
                }).await.unwrap();
        } else if args.len() == 2 {
            let ok = if args[0] == SETTINGS_CHOICES[0] {
                // prefix
                settings.prefix = args[1].to_string();
                Self::reply_success(msg.channel_id, ctx, &format!("prefix is now '{}'", settings.prefix))
                    .await
                    .unwrap();

                Ok(())
            } else if args[0] == SETTINGS_CHOICES[1] {
                // implicit-auto-embed

                match args[1].parse::<bool>() {
                    Ok(value) => {
                        settings.do_implicit_auto_embed = value;

                        if value {
                            Self::reply_success(msg.channel_id, ctx, "bot will now autoembed")
                                .await
                                .unwrap();
                        } else {
                            Self::reply_success(msg.channel_id, ctx, "bot will no longer autoembed")
                                .await
                                .unwrap();
                        }
                        Ok(())
                    },
                    Err(_) => {
                        Self::reply_error(msg.channel_id, ctx, "expected boolean")
                            .await
                            .unwrap();

                        Err(())
                    }
                }
            } else {
                Self::reply_error(msg.channel_id, ctx,"invalid setting")
                    .await
                    .unwrap();

                Err(())
            };

            if ok.is_ok() {
                let f = File::create(&self.settings_path).unwrap();
                serde_json::to_writer(f, &settings).unwrap();
            }
        } else {
            Self::reply_error(msg.channel_id, ctx, "required exactly 1 arg")
                .await
                .unwrap();
        }
    }
}

#[async_trait]
impl EventHandler for EmbedBot {

    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {

            let mut data = ctx.data.write().await;
            let mut settings = data.get_mut::<Settings>().unwrap();

            if msg.content.starts_with(&settings.prefix) {
                let commandline = &msg.content[settings.prefix.len()..].split(char::is_whitespace)
                    .collect::<Vec<&str>>();

                let cmd = commandline[0];
                let args = &commandline[1..];

                if commandline.is_empty() {
                    Self::reply_error(msg.channel_id, &ctx, "expected command")
                        .await
                        .unwrap();
                } else {
                    match cmd {
                        "embed"    => self.embed_subroutine(&ctx, &msg, &args[1], Some(&args[2])).await,
                        "settings" => self.settings_subroutine(&mut settings, &ctx, &msg, &args).await,
                        _ => {
                            msg.channel_id
                                .send_message(&ctx, |m| m.content(":x: Error unknown command"))
                                .await
                                .unwrap();
                        },
                    }
                }

            } else if settings.do_implicit_auto_embed {
                let content: Vec<_> = msg.content.split('\n').collect();

                let (url, comments) = match &content[..] {
                    [] => (None, None),
                    [a] => (is_url(a).then(|| a.to_string()), None),
                    args => {
                        let (urls, comments): (Vec<_>, Vec<_>) = args.iter()
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                            .partition(|a| is_url(a));

                        (urls.into_iter().next(), Some(comments.join("\n")))
                    }
                };

                if let Some(url) = url {
                    self.embed_subroutine(&ctx, &msg, &url, comments.as_deref())
                        .await;
                }
            }
        }
    }

    async fn ready(&self, _ctx: Context, _ready: Ready) {
        println!("[Info] logged in");
    }
}