use crate::post_grab_api;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub discord_token: String,
    pub modules: Option<Modules>,
}

impl Debug for Settings {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Settings")
            .field("discord_token", &"[REDACTED]")
            .field("modules", &self.modules)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Modules {
    #[cfg(feature = "reddit")]
    pub reddit: Option<post_grab_api::reddit::ApiSettings>,

    #[cfg(feature = "ninegag")]
    pub ninegag: Option<post_grab_api::ninegag::ApiSettings>,

    #[cfg(feature = "svg")]
    pub svg: Option<post_grab_api::svg::ApiSettings>,

    #[cfg(feature = "twitter")]
    pub twitter: Option<post_grab_api::twitter::ApiSettings>,
}
