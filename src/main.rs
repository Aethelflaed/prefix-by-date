mod application;
mod cli;
mod matcher;
mod processing;
mod replacement;
mod ui;

use application::{Application, Result};

fn main() -> Result<()> {
    let mut app = Application::new()?;
    app.setup()?;
    app.run()
}
