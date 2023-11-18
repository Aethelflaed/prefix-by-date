use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Default, Copy, Clone, ValueEnum)]
pub enum Interactive {
    #[default]
    Off,
    Text,
    Gui,
}

#[derive(Default, Parser)]
#[command(version)]
pub struct Cli {
    #[clap(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,

    /// Prefix by today's date
    #[arg(long, action)]
    pub today: bool,

    /// Prefix by time too
    #[arg(long, action)]
    pub time: bool,

    /// Start the program interactively or not
    #[arg(short, long, value_enum, default_value_t = Interactive::Off)]
    pub interactive: Interactive,

    pub paths: Vec<PathBuf>,
}
