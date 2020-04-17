use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use super::post_grab_api::*;


pub fn is_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}


fn get_post(api: &dyn PostScraper, url: &str) -> Result<Box<dyn Post>, Error> {

    match api.get_post(url) {
        Ok(post) => Ok(post),
        Err(_) if url.ends_with("#") => get_post(api, url.trim_end_matches("#")),
        Err(e) => Err(e)
    }
}



#[derive(Default)]
pub struct EmbedBot {
    apis: Vec<Box<dyn PostScraper + Send + Sync>>
}


impl EmbedBot {
    pub fn new() -> Self {
        Self::default()
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
}


impl EventHandler for EmbedBot {
    fn message(&self, context: Context, msg: Message) {

        if is_url(&msg.content) {
            match self.find_api(&msg.content) {
                Some(api) => match get_post(api, &msg.content) {
                    Ok(post) if post.should_embed() => {
                        msg.channel_id.send_message(&context, |m| {
                            post.create_embed(&msg.author, m);
                            m
                        }).expect("could not send msg");

                        msg.delete(context.http)
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

    fn ready(&self, _ctx: Context, _ready: Ready) {
        println!("[Info] logged in");
    }
}