use std::str::FromStr;

use clap::{AppSettings, Clap};
use strum::{EnumVariantNames, VariantNames};

enum SplitState {
    Default,
    InQuotes(char),
}

fn strip_quotes(s: &str) -> &str {
    const QUOTES: [char; 2] = ['\'', '"'];

    for &q in &QUOTES {
        if s.starts_with(q) && s.ends_with(q) {
            return &s[1..(s.len()-1)];
        }
    }

    s
}

pub fn command_line_split(cmdl: &str) -> impl Iterator<Item=&str> {
    let mut state = SplitState::Default;

    cmdl.split(move |ch| {
        match state {
            SplitState::Default => match ch {
                ' ' => true,
                '\'' | '"' => {
                    state = SplitState::InQuotes(ch);
                    false
                },
                _ => false
            },
            SplitState::InQuotes(quot) => {
                if ch == quot {
                    state = SplitState::Default;
                }
                false
            }
        }
    })
        .map(strip_quotes)
}

#[derive(Clap, Debug)]
#[clap(setting = AppSettings::NoBinaryName)]
pub enum EmbedBotOpts {
    Settings(SettingsSubcommand),
    Embed {
        url: String,

        #[clap(short = "c", long = "comment")]
        comment: Option<String>,
    }
}

#[derive(Clap, Debug)]
pub enum SettingsSubcommand {
    Set {
        key: SettingsOptions,
        value: String,
    },
    Get {
        key: SettingsOptions
    },
}


#[derive(Clap, Debug, EnumVariantNames)]
pub enum SettingsOptions {
    DoImplicitAutoEmbed,
    Prefix
}


impl FromStr for SettingsOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DoImplicitAutoEmbed" => Ok(SettingsOptions::DoImplicitAutoEmbed),
            "Prefix" => Ok(SettingsOptions::Prefix),
            _ => Err(format!("settings option must be in {:?}", SettingsOptions::VARIANTS))
        }
    }
}