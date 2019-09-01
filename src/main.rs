#![feature(try_trait, bind_by_move_pattern_guards)]

mod post_grab_api;
use post_grab_api::*;

use discord::model::{Event, Message};
use discord::{Discord, State};
use std::io::Read;

fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

fn choose_grab_api(url: &str) -> Option<Box<dyn post_grab_api::PostGrabAPI>> {
    if url.starts_with("https://www.reddit.com") {
        Some(Box::new(post_grab_api::reddit::RedditAPI::default()))
    } else if url.starts_with("https://9gag.com") {
        Some(Box::new(post_grab_api::ninegag::NineGagAPI::default()))
    } else {
        None
    }
}

fn escape_title(title: &str) -> String {
    title
        .replace("*", "\\*")
        .replace("_", "\\_")
        .replace("~", "\\~")
        .replace(">", "\\>")
}

fn should_embed(post: &Post) -> bool {
    (&post.website == "9gag" && post.post_type == PostType::Video)
        || (&post.website == "reddit" && post.post_type == PostType::Image)
}

fn send_embed_message(
    dcord: &Discord,
    msg: &Message,
    post: &Post,
) -> Result<Message, discord::Error> {
    match post.post_type {
        PostType::Image => dcord.send_embed(msg.channel_id, "", |embed_builder| {
            embed_builder
                .title(&format!("'{}'", escape_title(&post.title)))
                .description(&post.origin)
                .author(|author_builder| author_builder.name(&msg.author.name))
                .url(&msg.content)
                .image(&post.embed_url)
        }),
        PostType::Video => dcord.send_message(
            msg.channel_id,
            &format!(
                ">>> Sender: **{}**\nSource: <{}>\nEmbedURL: {}\n\n**\"{}\"**",
                msg.author.name,
                msg.content,
                post.embed_url,
                escape_title(&post.title)
            ),
            "",
            false,
        ),
    }
}

fn main() {
    let discord = {
        let mut tokfile = std::fs::File::open("/etc/embedbot.conf").expect("could not open token file");

        let mut buf = String::new();
        tokfile.read_to_string(&mut buf);

        Discord::from_bot_token(&buf).expect("login failed")
    };

    let (mut connection, _) = discord.connect().expect("connect failed");

    println!("[Info] logged in");

    loop {
        let event = match connection.recv_event() {
            Ok(event) => event,
            Err(discord::Error::Closed(code, body)) => {
                println!("[Error] Connection closed with status {:?}: {}", code, body);
                break;
            }
            _ => continue,
        };

        match event {
            Event::MessageCreate(msg) if is_url(&msg.content) => {
                match choose_grab_api(&msg.content) {
                    Some(mut api) => match api.get_post(&msg.content) {
                        Ok(post) => {
                            if should_embed(&post) {
                                send_embed_message(&discord, &msg, &post)
                                    .expect("could not send msg");

                                discord
                                    .delete_message(msg.channel_id, msg.id)
                                    .expect("could not delete msg");
                            } else {
                                println!(
                                    "[Info] ignoring '{}'. Reason: not supposed to embed",
                                    msg.content
                                )
                            }
                        }
                        Err(e) => eprintln!("[Error] could not revc post at: {:?}", e),
                    },
                    None => println!("[Info] ignoring '{}'. Reason: no API found", msg.content),
                }
            }
            _ => (),
        }
    }
}
