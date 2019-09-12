use super::*;

#[derive(Default)]
pub struct NineGagAPI;

impl PostGrabAPI for NineGagAPI {
    fn get_post(&mut self, url: &str) -> Result<Post, Error> {
        let html = wget_html(url, USER_AGENT)?;

        let title: String = {
            let title_selector = scraper::Selector::parse("title").unwrap();
            html.select(&title_selector).next()?.text().collect()
        };

        let build_json: serde_json::Value = {
            let script_selector = scraper::Selector::parse("script").unwrap();

            let script_text: String = html
                .select(&script_selector)
                .find(|elem| elem.text().collect::<String>().contains("JSON.parse"))?
                .text()
                .collect::<String>()
                .replace("\\", "");

            serde_json::from_str(&script_text[29..(script_text.len() - 3)])?
        };

        let post_json = build_json
            .as_object()?
            .get("data")?
            .as_object()?
            .get("post")?
            .as_object()?;

        let post_type_str = post_json.get("type")?.as_str()?;

        let embed_url = match post_type_str {
            "Photo" => post_json
                .get("images")?
                .as_object()?
                .get("image700")?
                .as_object()?
                .get("url")?
                .as_str()?,

            "Animated" => post_json
                .get("images")?
                .as_object()?
                .get("image460svwm")?
                .as_object()?
                .get("url")?
                .as_str()?,

            _ => post_json.get("vp9Url")?.as_str()?,
        }
        .to_string();

        let post_type = if post_type_str == "Photo" {
            PostType::Image
        } else {
            PostType::Video
        };

        Ok(Post {
            website: "9gag".to_string(),
            origin: "9gag".to_string(),
            text: "".to_string(),
            title: title[0..(title.len() - 7)].to_string(), // remove ' - 9GAG' from end
            embed_url,
            post_type,
        })
    }
}
