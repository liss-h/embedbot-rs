use clap::{AppSettings, ArgEnum, Clap, Subcommand};
use std::str::FromStr;
use strum::AsStaticStr;

enum SplitState {
    Default,
    InQuotes(char),
}

fn strip_quotes(s: &str) -> &str {
    const QUOTES: [char; 2] = ['\'', '"'];

    for &q in &QUOTES {
        if s.starts_with(q) && s.ends_with(q) {
            return &s[1..(s.len() - 1)];
        }
    }

    s
}

pub fn command_line_split(cmdl: &str) -> impl Iterator<Item = &str> {
    let mut state = SplitState::Default;

    cmdl.split(move |ch| match state {
        SplitState::Default => match ch {
            ' ' => true,
            '\'' | '"' => {
                state = SplitState::InQuotes(ch);
                false
            }
            _ => false,
        },
        SplitState::InQuotes(quot) => {
            if ch == quot {
                state = SplitState::Default;
            }
            false
        }
    })
    .map(strip_quotes)
}

#[derive(Clap, Debug)]
#[clap(setting = AppSettings::NoBinaryName)]
pub enum EmbedBotOpts {
    #[clap(flatten, about = "Change or view the bot settings")]
    Settings(SettingsSubcommand),
    Embed {
        url: String,

        #[clap(short = 'c', long = "comment")]
        comment: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
#[clap(setting = AppSettings::SubcommandRequired)]
pub enum SettingsSubcommand {
    #[clap(about = "Sets a bot setting to a new value")]
    Set {
        #[clap(about = "the setting to change", possible_values = SettingsOptions::VARIANTS)]
        key: SettingsOptions,

        #[clap(about = "the desired value")]
        value: String,
    },

    #[clap(about = "Displays the current value of a setting")]
    Get {
        #[clap(about = "The setting value to display", possible_values = SettingsOptions::VARIANTS)]
        key: SettingsOptions,
    },
}

#[derive(ArgEnum, Debug, AsStaticStr, PartialEq)]
#[strum(serialize_all = "kebab-case")]
pub enum SettingsOptions {
    DoImplicitAutoEmbed,
    Prefix,
}

impl FromStr for SettingsOptions {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        <Self as ArgEnum>::from_str(s, true)
    }
}
