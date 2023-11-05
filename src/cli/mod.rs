use clap::Parser;
use std::path::PathBuf;

#[cfg(feature = "cli")]
mod reporter;
#[cfg(feature = "cli")]
pub use reporter::Reporter;

#[derive(Parser)]
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

impl Cli {
    pub fn setup() -> Self {
        Cli::parse()
    }
}
