use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use serde::{Deserialize, Serialize};

use super::post_grab_api::*;
use std::fs::File;
use std::path::{PathBuf, Path};

const SETTINGS_CHOICES: [&str; 2] = ["prefix", "do-implicit-auto-embed"];
const SETTINGS_CHOICES_DESCR: [&str; 2] = [":exclamation: prefix", ":envelope: do-implicit-auto-embed"];


pub fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}


fn get_post(api: &dyn PostScraper, url: &str) -> Result<Box<dyn Post>, Error> {

    match api.get_post(url) {
        Ok(post) => Ok(post),
        Err(_) if url.ends_with('#') => get_post(api, url.trim_end_matches('#')),
        Err(e) => Err(e)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub prefix: String,
    pub do_implicit_auto_embed: bool,
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
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    apis: Vec<Box<dyn PostScraper + Send + Sync>>,
}


impl EmbedBot {
    pub fn new<P: AsRef<Path>>(settings_path: P, settings: Settings) -> Self {
        EmbedBot {
            settings: Mutex::new(settings),
            settings_path: settings_path.as_ref().to_path_buf(),
            apis: Vec::new(),
        }
    }

    pub fn find_api(&self, url: &str) -> Option<&dyn PostScraper> {
        self.apis
            .iter()
            .find(|a| a.is_suitable(url))
            .map(|a| a.as_ref() as &dyn PostScraper)
    }

    pub fn register_api<T: 'static + PostScraper + Send + Sync>(&mut self, api: T) {
        self.apis.push(Box::new(api));
    }

    fn embed_subroutine(&self, ctx: &Context, msg: &Message, args: &[&str]) {
        if args.len() == 1 && is_url(args[0]) {
            match self.find_api(args[0]) {
                Some(api) => match get_post(api, args[0]) {
                    Ok(post) if post.should_embed() => {
                        msg.channel_id.send_message(ctx, |m| {
                            post.create_embed(&msg.author, m);
                            m
                        }).expect("could not send msg");

                        msg.delete(ctx)
                            .expect("could not delete msg");

                        println!("[Info] embedded '{}': {:?}", msg.content, post);
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
    }


    fn settings_subroutine(&self, settings: &mut Settings, ctx: &Context, msg: &Message, args: &[&str]) {
        let reply_sucess = |mes: &str| {
            msg.channel_id
                .send_message(&ctx, |mb| mb.content(format!(":white_check_mark: Success: {}", mes)))
                .unwrap();
        };

        let reply_err = |mes: &str| {
            msg.channel_id
                .send_message(&ctx, |mb| mb.content(format!(":x: Error: {}", mes)))
                .unwrap();
        };


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
                }).unwrap();
        } else if args.len() == 2 {
            let ok = if args[0] == SETTINGS_CHOICES[0] {
                // prefix
                settings.prefix = args[1].to_string();
                reply_sucess(&format!("prefix is now '{}'", settings.prefix));
                Ok(())
            } else if args[0] == SETTINGS_CHOICES[1] {
                // implicit-auto-embed

                match args[1].parse::<bool>() {
                    Ok(value) => {
                        settings.do_implicit_auto_embed = value;

                        if value {
                            reply_sucess("bot will now autoembed");
                        } else {
                            reply_sucess("bot will no longer autoembed");
                        }
                        Ok(())
                    },
                    Err(_) => {
                        reply_err("expected boolean");
                        Err(())
                    }
                }
            } else {
                reply_err("invalid setting");
                Err(())
            };

            if ok.is_ok() {
                let f = File::create(&self.settings_path).unwrap();
                serde_json::to_writer(f, &settings).unwrap();
            }
        } else {
            reply_err("required exactly 1 arg");
        }
    }
}


impl EventHandler for EmbedBot {

    fn message(&self, ctx: Context, msg: Message) {
        if !msg.author.bot {

            let mut settings = self.settings.lock();
            let commandline = &msg.content[settings.prefix.len()..].split(' ')
                .collect::<Vec<&str>>();

            let cmd = commandline[0];
            let args = &commandline[1..];

            if msg.content.starts_with(&settings.prefix) {
                if commandline.is_empty() {
                    msg.channel_id
                        .send_message(&ctx, |m| m.content("Error: expected command"))
                        .unwrap();
                } else {
                    match cmd {
                        "embed"    => self.embed_subroutine(&ctx, &msg, &args[..]),
                        "settings" => self.settings_subroutine(&mut settings, &ctx, &msg, &args[..]),
                        _ => (),
                    }
                }

            } else if settings.do_implicit_auto_embed {
                self.embed_subroutine(&ctx, &msg, &[&msg.content]);
            }
        }
    }

    fn ready(&self, _ctx: Context, _ready: Ready) {
        println!("[Info] logged in");
    }
}