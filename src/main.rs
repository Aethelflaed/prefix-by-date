mod cli;
mod file;
mod log_config;
mod matcher;
mod replacement;
mod state;

use cli::Cli;
use file::prefix_file_if_possible;
use state::State;

fn main() -> std::io::Result<()> {
    log_config::setup()?;

    let cli = Cli::setup();
    log::set_max_level(cli.verbose.log_level_filter());

    let state = State::from(&cli)?;

    for file in &cli.files {
        prefix_file_if_possible(file, &state)?;
    }

    Ok(())
}
