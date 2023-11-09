mod cli;
mod application;
mod matcher;
mod processing;
mod replacement;
mod reporter;

use cli::Cli;
use application::{Application, Result};
use processing::Processing;

fn main() -> Result<()> {
    let mut app = Application::new()?;
    let cli = Cli::setup();

    app.setup(&cli)?;

    Processing::new(&app).run(&cli.paths)?;

    Ok(())
}
