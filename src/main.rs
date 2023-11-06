mod cli;
mod context;
mod matcher;
mod processing;
mod replacement;
mod reporter;

use cli::Cli;
use context::{Context, Result};
use processing::Processing;

fn main() -> Result<()> {
    let mut context = Context::new()?;
    let cli = Cli::setup();

    context.setup(&cli)?;


    Processing::new(&context).run(&cli.paths)?;

    Ok(())
}
