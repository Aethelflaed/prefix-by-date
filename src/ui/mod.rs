use crate::application::Result;
use crate::matcher::Matcher;
use crate::processing::{Communication, Confirmation, Error, Processing};
use crate::replacement::Replacement;

use std::boxed::Box;
use std::path::{Path, PathBuf};

use env_logger::Builder;
type LogResult = std::result::Result<(), log::SetLoggerError>;

mod gui;
mod text;

#[cfg(feature = "cli")]
pub use text::Text;
#[cfg(not(feature = "cli"))]
pub use NonInteractive as Text;

#[cfg(feature = "gui")]
pub use gui::Gui;
#[cfg(not(feature = "gui"))]
pub use NonInteractive as Gui;

pub trait Interface: Send {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult;

    fn confirm(&self, replacement: &Replacement) -> Confirmation;

    fn process(
        &mut self,
        matchers: &Vec<Box<dyn Matcher>>,
        paths: &Vec<PathBuf>,
    ) -> Result<()>;
}

pub struct NonInteractive {}

impl NonInteractive {
    pub fn new() -> Self {
        NonInteractive {}
    }
}

impl Interface for NonInteractive {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        logger_builder.try_init()
    }

    fn confirm(&self, _replacement: &Replacement) -> Confirmation {
        Confirmation::Accept
    }

    fn process(
        &mut self,
        matchers: &Vec<Box<dyn Matcher>>,
        paths: &Vec<PathBuf>,
    ) -> Result<()> {
        Processing::new(self, &matchers, &paths).run()?;
        Ok(())
    }
}

impl Communication for NonInteractive {
    fn processing(&self, _path: &Path) {}
    fn processing_ok(&self, _replacement: &Replacement) {}
    fn processing_err(&self, _path: &Path, _error: &Error) {}
    fn confirm(&self, replacement: &Replacement) -> Confirmation {
        Interface::confirm(self, replacement)
    }
}
