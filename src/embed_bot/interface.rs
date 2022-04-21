use clap::{ArgEnum, Parser, Subcommand};

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

#[derive(Parser, Debug)]
#[clap(no_binary_name = true)]
pub enum EmbedBotOpts {
    /// change or view the bot settings
    #[clap(flatten)]
    Settings(SettingsSubcommand),

    /// embed a post
    Embed {
        url: String,

        /// a personal comment to include
        #[clap(short, long)]
        comment: Option<String>,

        /// embed fully even if post is flagged as nsfw
        #[clap(long)]
        ignore_nsfw: bool,
    },
}

#[derive(Subcommand, Debug)]
#[clap(subcommand_required = true)]
pub enum SettingsSubcommand {
    /// Sets a bot setting to a new value
    Set {
        /// the setting to change
        #[clap(arg_enum)]
        key: SettingsOptions,

        /// the desired value
        value: String,
    },

    /// display the current value of a setting
    Get {
        /// the setting value to display
        #[clap(arg_enum)]
        key: SettingsOptions,
    },
}

#[derive(ArgEnum, Debug, PartialEq, Clone, Copy)]
pub enum SettingsOptions {
    DoImplicitAutoEmbed,
    Prefix,
}
