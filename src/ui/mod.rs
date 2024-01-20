use crate::application::{Interactive, Result};
use crate::matcher::Matcher;
use crate::processing::{
    self, Communication, Confirmation, Error, Processing, Reporter,
};
use crate::replacement::Replacement;

use std::boxed::Box;
use std::path::{Path, PathBuf};

use env_logger::Builder;
type LogResult = std::result::Result<(), log::SetLoggerError>;

mod actions;
mod state;

mod gui;
mod text;

#[cfg(feature = "text")]
pub use text::Text;
#[cfg(not(feature = "text"))]
pub use NonInteractive as Text;

#[cfg(feature = "gui")]
pub use gui::Gui;
#[cfg(not(feature = "gui"))]
pub use NonInteractive as Gui;

pub trait Interface: Send {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        logger_builder.try_init()
    }

    fn process(
        &mut self,
        matchers: &[Box<dyn Matcher>],
        paths: &[PathBuf],
    ) -> Result<()>;
}

pub fn from(interactive: Interactive) -> Box<dyn Interface> {
    match interactive {
        Interactive::Text if cfg!(feature = "text") && Text::available() => {
            Box::new(Text::new())
        }
        Interactive::Gui if cfg!(feature = "gui") => Box::new(Gui::new()),
        _ => Box::new(NonInteractive::new()),
    }
}

pub struct NonInteractive {}

impl NonInteractive {
    #[allow(dead_code)]
    /// Inidcate whether or not this interface is available
    pub fn available() -> bool {
        true
    }

    pub fn new() -> Self {
        NonInteractive {}
    }
}

impl Interface for NonInteractive {
    fn process(
        &mut self,
        matchers: &[Box<dyn Matcher>],
        paths: &[PathBuf],
    ) -> Result<()> {
        Processing::new(self, matchers, paths).run()?;
        Ok(())
    }
}

impl Reporter for NonInteractive {
    fn setup(&self, _count: usize) {}
    fn processing(&self, _path: &Path) {}
    fn processing_ok(&self, _replacement: &Replacement) {}
    fn processing_err(&self, _path: &Path, _error: &Error) {}
}

impl Communication for NonInteractive {
    fn confirm(&self, _replacement: &Replacement) -> Confirmation {
        Confirmation::Accept
    }
    fn rescue(&self, error: Error) -> processing::Result<Replacement> {
        Err(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{matchers, with_temp_dir, test, assert_fs::*};
    use predicates::prelude::*;

    #[test]
    fn from_different_interactive_values() {
        from(Interactive::Gui);
        from(Interactive::Text);
        from(Interactive::Off);
    }

    #[test]
    fn non_interactive_available() {
        assert!(NonInteractive::available());
    }

    #[test]
    fn non_interactive_run() {
        let matchers = [matchers::ymd_boxed()];

        with_temp_dir(|temp| {
            let child1 = temp.existing_child("foo 20240120");
            let child2 = temp.existing_child("bar 2024012");

            let paths = [
                child1.to_path_buf(), child2.to_path_buf()
            ];
            let mut ui = NonInteractive::new();

            assert!(ui.process(&matchers, &paths).is_ok());

            child1.assert(predicate::path::missing());
            temp.child("2024-01-20 foo").assert(predicate::path::exists());

            child2.assert(predicate::path::exists());
        });
    }
}
