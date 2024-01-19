mod application;
mod matcher;
mod processing;
mod replacement;
mod ui;

#[cfg(test)]
mod test;

use application::{Application, Result};

fn main() -> Result<()> {
    let mut app = Application::new();
    app.setup()?;
    app.run()
}
