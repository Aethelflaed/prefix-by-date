mod cli;
mod log_config;
mod matcher;
mod processing;
mod replacement;
mod reporter;
mod state;

use cli::Cli;
use processing::{Processing, Result};
use state::State;

#[cfg(not(feature = "cli"))]
fn main() -> Result<()> {
    log_config::setup()?;

    let cli = Cli::setup();
    log::set_max_level(cli.verbose.log_level_filter());

    let state = State::from(&cli)?;
    Processing::new(&state, &cli.paths).run()
}

#[cfg(feature = "cli")]
fn main() -> Result<()> {
    log_config::setup()?;

    let cli = Cli::setup();
    log::set_max_level(cli.verbose.log_level_filter());

    let reporter = cli::Reporter::default();

    let mut state = State::from(&cli)?;
    state.add_reporter(Box::new(reporter.clone()));

    Processing::new(&state, &cli.paths).run()?;

    reporter.bar.finish();
    Ok(())
}
