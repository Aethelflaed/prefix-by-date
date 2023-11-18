use clap::Parser;
use std::path::PathBuf;

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

    pub paths: Vec<PathBuf>,
}
