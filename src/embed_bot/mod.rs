pub mod interface;

use super::post_grab_api::PostScraper;
use interface::{command_line_split, EmbedBotOpts, SettingsOptions, SettingsSubcommand};

use clap::Clap;
use serde::{Deserialize, Serialize};
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{channel::Message, gateway::Ready, id::ChannelId, user::User},
    prelude::TypeMapKey,
};
use std::{
    collections::HashSet,
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
};
use strum::AsStaticRef;
use url::Url;

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum PostType {
    Text,
    Gallery,
    Image,
    Video,
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Crossposted {
    Crosspost,
    NonCrosspost,
    Any,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RedditEmbedSet(pub HashSet<(PostType, Crossposted)>);

#[derive(Serialize, Deserialize, Debug)]
#[repr(transparent)]
pub struct NineGagEmbedSet(pub HashSet<PostType>);

#[derive(Serialize, Deserialize, Debug)]
#[repr(transparent)]
pub struct DefaultTrueBool(pub bool);

impl Default for RedditEmbedSet {
    fn default() -> Self {
        RedditEmbedSet(
            [
                (PostType::Text, Crossposted::Any),
                (PostType::Gallery, Crossposted::Any),
                (PostType::Image, Crossposted::Any),
                (PostType::Video, Crossposted::Crosspost),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl Default for NineGagEmbedSet {
    fn default() -> Self {
        NineGagEmbedSet([PostType::Video].into_iter().collect())
    }
}

impl Default for DefaultTrueBool {
    fn default() -> Self {
        DefaultTrueBool(true)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct EmbedSettings {
    #[serde(default)]
    pub reddit: RedditEmbedSet,

    #[serde(default)]
    pub ninegag: NineGagEmbedSet,

    #[serde(default)]
    pub imgur: DefaultTrueBool,

    #[serde(default)]
    pub svg: DefaultTrueBool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub prefix: String,
    pub do_implicit_auto_embed: bool,

    #[serde(default)]
    pub embed_settings: EmbedSettings,
}

impl Settings {
    pub fn display_value(&self, opt: &SettingsOptions) -> &dyn Display {
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
            embed_settings: EmbedSettings::default(),
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

    pub fn find_api(&self, url: &Url) -> Option<&(dyn PostScraper + Send + Sync)> {
        self.apis
            .iter()
            .find(|a| a.is_suitable(url))
            .map(AsRef::as_ref)
    }

    pub fn register_api<T: 'static + PostScraper + Send + Sync>(&mut self, api: T) {
        self.apis.push(Box::new(api));
    }

    async fn embed(
        &self,
        ctx: &Context,
        chan: ChannelId,
        author: &User,
        url: Url,
        comment: Option<&str>,
        settings: &Settings,
    ) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        if let Some(api) = self.find_api(&url) {
            let url = {
                let mut u = url.clone();
                u.set_fragment(None);
                u
            };

            match api.get_post(url.clone()).await {
                Ok(post) if post.should_embed(settings) => {
                    let msg = post.send_embed(author, comment, chan, ctx).await?;

                    println!("[Info] embedded '{}': {:?}", url, post);
                    Ok(Some(msg))
                }
                Ok(_) => {
                    println!("[Info] ignoring '{}'. Reason: not supposed to embed", url);
                    Ok(None)
                }
                Err(e) => {
                    eprintln!("[Error] could not fetch post. Reason: {:?}", e);
                    Ok(None)
                }
            }
        } else {
            println!("[Info] ignoring '{}'. Reason: no api available", url);
            Ok(None)
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
                    msg.content.trim_start_matches(&settings.prefix),
                ))
                .map_err(|e| format!("```{}```", e));

                match opts {
                    Ok(EmbedBotOpts::Embed { url, comment }) => match Url::parse(&url) {
                        Ok(url) => {
                            self.embed(
                                &ctx,
                                msg.channel_id,
                                &msg.author,
                                url,
                                comment.as_deref(),
                                settings,
                            )
                            .await
                            .unwrap();
                        }
                        Err(_) => {
                            Self::reply_error(
                                msg.channel_id,
                                &ctx,
                                &format!("could not parse url: {}", url),
                            )
                            .await
                            .unwrap();
                        }
                    },
                    Ok(EmbedBotOpts::Settings(SettingsSubcommand::Get { key })) => {
                        Self::reply_success(
                            msg.channel_id,
                            &ctx,
                            &format!(
                                "```c\n{key} == {value}\n```",
                                key = key.as_static(),
                                value = settings.display_value(&key)
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
                                Self::reply_error(msg.channel_id, &ctx, &e).await.unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        Self::reply_error(msg.channel_id, &ctx, &e).await.unwrap();
                    }
                }
            } else if settings.do_implicit_auto_embed {
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
                        // think about this: .filter(|u| self.find_api(u).is_some());

                        let comments = comments.into_iter().intersperse("\n").collect::<String>();

                        (urls.next(), Some(comments))
                    }
                };

                if let Some(url) = url {
                    let reply = self
                        .embed(
                            &ctx,
                            msg.channel_id,
                            &msg.author,
                            url,
                            comment.as_deref(),
                            settings,
                        )
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
