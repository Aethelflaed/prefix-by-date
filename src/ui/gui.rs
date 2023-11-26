#![cfg(feature = "gui")]

use crate::application::{Application, Confirmation};
use crate::cli::Cli;
use crate::matcher::Matcher;
use crate::replacement::Replacement;
use crate::ui::Interface;

use std::boxed::Box;
use std::path::Path;

use env_logger::Builder;

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Gui {}

impl Gui {
    pub fn new() -> Self {
        Gui {}
    }
}

impl Interface for Gui {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        logger_builder.try_init()
    }
    fn after_setup(&mut self, _cli: &Cli, _matchers: &[Box<dyn Matcher>]) {}
    fn after_process(&self, _path: &Path) {}

    fn confirm(
        &self,
        _app: &Application,
        _replacement: &Replacement,
    ) -> Confirmation {
        Confirmation::Abort
    }
}
