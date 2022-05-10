use crate::post_grab_api;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeSettings {
    #[cfg(feature = "implicit-auto-embed")]
    pub do_implicit_auto_embed: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitSettings {
    pub discord_token: String,
    pub modules: Option<Modules>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Modules {
    #[cfg(feature = "reddit")]
    pub reddit: Option<post_grab_api::reddit::ApiSettings>,

    #[cfg(feature = "ninegag")]
    pub ninegag: Option<post_grab_api::ninegag::ApiSettings>,

    #[cfg(feature = "svg")]
    pub svg: Option<post_grab_api::svg::ApiSettings>,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            #[cfg(feature = "implicit-auto-embed")]
            do_implicit_auto_embed: true,
        }
    }
}
