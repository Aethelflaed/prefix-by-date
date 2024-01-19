use std::path::PathBuf;

use clap::{builder::ArgAction, Parser, ValueEnum};

#[derive(Default, Debug, Copy, Clone, ValueEnum)]
pub enum Interactive {
    #[default]
    Off,
    Text,
    Gui,
}

#[derive(Default, Debug, Copy, Clone, ValueEnum)]
pub enum Metadata {
    #[default]
    None,
    Created,
    Modified,
    Both,
}

impl Metadata {
    pub fn created(&self) -> bool {
        matches!(self, Self::Created | Self::Both)
    }

    pub fn modified(&self) -> bool {
        matches!(self, Self::Modified | Self::Both)
    }
}

/// Prefix files by date
#[derive(Default, Debug, Parser)]
#[command(version)]
pub struct Cli {
    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Sets a custom config directory
    ///
    /// The default value is $PREFIX_BY_DATE_CONFIG if it is set, or
    /// $XDG_CONFIG_HOME/prefix-by-date otherwise
    #[arg(short = 'C', long, value_name = "DIR")]
    pub config: Option<PathBuf>,

    /// Prefix by today's date
    #[arg(long)]
    pub today: bool,

    /// Prefix by date and time
    #[arg(long = "time", overrides_with = "time")]
    pub no_time: bool,

    /// Only prefix by date
    #[arg(long = "no-time", action = ArgAction::SetFalse)]
    pub time: bool,

    /// Start the program interactively or not
    #[arg(short, long, value_enum, default_value_t = Interactive::Off)]
    pub interactive: Interactive,

    /// Metadata matchers to enable
    #[arg(short, long, value_enum)]
    pub metadata: Option<Metadata>,

    /// Paths to process
    pub paths: Vec<PathBuf>,
}

impl Cli {
    /// Indicate if the user wants to prefix the time along the date or not.
    ///
    /// A value of None indicates no preference.
    ///
    /// The boolean negation flag is inspired by
    /// https://jwodder.github.io/kbits/posts/clap-bool-negate/
    ///
    /// The two field have different values if nothing has been specified,
    /// which we detect to return None
    pub fn time(&self) -> Option<bool> {
        if self.time != self.no_time {
            None
        } else {
            Some(self.time)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{test, assert_eq};

    #[test]
    fn time_and_no_time() {
        let mut args = vec!["arg0"];
        assert!(Cli::parse_from(&args).time().is_none());

        args.push("--time");
        assert_eq!(Some(true), Cli::parse_from(&args).time());

        args.push("--no-time");
        assert_eq!(Some(false), Cli::parse_from(&args).time());

        let args = vec!["arg0", "--no-time"];
        assert_eq!(Some(false), Cli::parse_from(&args).time());
    }
}
