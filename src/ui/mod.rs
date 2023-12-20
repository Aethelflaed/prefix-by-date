use crate::application::Result;
use crate::cli::Interactive;
use crate::matcher::Matcher;
use crate::processing::{Communication, Confirmation, Error, Processing};
use crate::replacement::Replacement;

use std::boxed::Box;
use std::path::{Path, PathBuf};

use env_logger::Builder;
type LogResult = std::result::Result<(), log::SetLoggerError>;

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
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult;

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
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        logger_builder.try_init()
    }

    fn process(
        &mut self,
        matchers: &[Box<dyn Matcher>],
        paths: &[PathBuf],
    ) -> Result<()> {
        Processing::new(self, matchers, paths).run()?;
        Ok(())
    }
}

impl Communication for NonInteractive {
    fn processing(&self, _path: &Path) {}
    fn processing_ok(&self, _replacement: &Replacement) {}
    fn processing_err(&self, _path: &Path, _error: &Error) {}
    fn confirm(&self, _replacement: &Replacement) -> Confirmation {
        Confirmation::Accept
    }
}
