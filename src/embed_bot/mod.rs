mod settings;

use crate::post_grab_api::{CreateResponse, DynPostScraper, EmbedOptions, Error, Post};
use itertools::Itertools;
use serenity::{
    async_trait,
    builder::{
        CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
        CreateMessage,
    },
    client::{Context, EventHandler},
    model::{
        application::{Command, CommandData, CommandOptionType, CommandType, Interaction},
        channel::Message,
        gateway::Ready,
    },
};
pub use settings::Settings;
use url::Url;

#[derive(Default)]
pub struct EmbedBot {
    apis: Vec<Box<dyn DynPostScraper + Send + Sync>>,
}

impl EmbedBot {
    pub fn new() -> Self {
        EmbedBot { apis: Vec::new() }
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

    fn reply_error(msg: &str, response: CreateResponse) -> CreateResponse {
        response.embed(CreateEmbed::new().title(":x: Error").description(msg))
    }
}

#[async_trait]
impl EventHandler for EmbedBot {
    #[cfg(feature = "implicit-auto-embed")]
    async fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {
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

                    let comments: String = Itertools::intersperse(comments.into_iter(), "\n").collect();

                    (urls.next(), Some(comments))
                },
            };

            if let Some(url) = url {
                match self.get_post(url.clone()).await {
                    Ok(post) => {
                        msg.channel_id
                            .send_message(
                                &ctx,
                                post.create_embed(
                                    &msg.author,
                                    &EmbedOptions { comment, ..Default::default() },
                                    CreateResponse::Message(CreateMessage::new()),
                                )
                                .into_message(),
                            )
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
                        tracing::error!("error while trying to embed {}: {}", url, e);
                    },
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, _ready: Ready) {
        let _ = Command::create_global_command(
            &ctx,
            CreateCommand::new("embed")
                .kind(CommandType::ChatInput)
                .description("embed a post")
                .add_option(
                    CreateCommandOption::new(CommandOptionType::String, "url", "url of the post").required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "ignore-nsfw",
                        "embed fully even if post is flagged as nsfw",
                    )
                    .required(false),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::Boolean,
                        "ignore-spoiler",
                        "embed fully even if post is flagged as spoiler",
                    )
                    .required(false),
                )
                .add_option(
                    CreateCommandOption::new(CommandOptionType::String, "comment", "a personal comment to include")
                        .required(false),
                ),
        )
        .await
        .unwrap();

        tracing::info!("logged in");
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(command) = &interaction {
            match &command.data {
                CommandData { name, options, .. } if name == "embed" => {
                    let url = options
                        .iter()
                        .find(|c| c.name == "url")
                        .unwrap()
                        .value
                        .as_str()
                        .unwrap();

                    let comment = options
                        .iter()
                        .find(|c| c.name == "comment")
                        .and_then(|c| c.value.as_str())
                        .map(|c| c.to_owned());

                    let ignore_nsfw = options
                        .iter()
                        .find(|c| c.name == "ignore-nsfw")
                        .and_then(|c| c.value.as_bool())
                        .unwrap_or(false);

                    let ignore_spoiler = options
                        .iter()
                        .find(|c| c.name == "ignore-spoiler")
                        .and_then(|c| c.value.as_bool())
                        .unwrap_or(false);

                    let opts = EmbedOptions { comment, ignore_nsfw, ignore_spoiler };

                    match Url::parse(url) {
                        Ok(url) => {
                            let user = &command.user;

                            match self.get_post(url.clone()).await {
                                Ok(post) => {
                                    command
                                        .create_response(&ctx, CreateInteractionResponse::Message({
                                            post.create_embed(user, &opts, CreateResponse::Interaction(CreateInteractionResponseMessage::new())).into_interaction()
                                        }))
                                        .await
                                        .unwrap();

                                    tracing::trace!("embedded '{}': {:?}", url, post);
                                },
                                Err(e) => {
                                    let msg = format!("{}", e);
                                    tracing::error!("error: {msg}");

                                    command
                                        .create_response(&ctx, CreateInteractionResponse::Message({
                                            Self::reply_error(&msg, CreateResponse::Interaction(CreateInteractionResponseMessage::new()))
                                                .into_interaction()
                                        }))
                                        .await
                                        .unwrap();
                                },
                            }
                        },
                        Err(_) => {
                            command
                                .create_response(
                                    &ctx,
                                    CreateInteractionResponse::Message({
                                        Self::reply_error(
                                            &format!("Could not parse url: {}", url),
                                            CreateResponse::Interaction(CreateInteractionResponseMessage::new()),
                                        )
                                        .into_interaction()
                                    }),
                                )
                                .await
                                .unwrap();
                        },
                    }
                },
                _ => (),
            }
        }
    }
}
