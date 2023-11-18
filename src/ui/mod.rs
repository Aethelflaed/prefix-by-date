use crate::application::Confirmation;
use crate::cli::Cli;
use crate::replacement::Replacement;

use std::path::Path;

use env_logger::Builder;
type LogResult = std::result::Result<(), log::SetLoggerError>;

pub mod text;

pub trait Interface {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult;
    fn after_setup(&mut self, cli: &Cli);
    fn after_process(&self, path: &Path);

    fn confirm(&self, path: &Path, replacement: &Replacement) -> Confirmation;
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
    fn after_setup(&mut self, _cli: &Cli) {}
    fn after_process(&self, _path: &Path) {}

    fn confirm(
        &self,
        _path: &Path,
        _replacement: &Replacement,
    ) -> Confirmation {
        Confirmation::Accept
    }
}
