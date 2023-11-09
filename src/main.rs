mod application;
mod cli;
mod matcher;
mod processing;
mod replacement;
mod reporter;

use application::{Application, Result};
use cli::Cli;
use processing::Processing;

fn main() -> Result<()> {
    let mut app = Application::new()?;
    let cli = Cli::setup();

    app.setup(&cli)?;

    Processing::new(&app).run(&cli.paths)?;

    Ok(())
}
