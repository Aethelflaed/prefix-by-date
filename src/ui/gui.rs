#![cfg(feature = "gui")]

use crate::application::{Confirmation, Result};
use crate::matcher::Matcher;
use crate::processing::{Communication, Error, Processing};
use crate::replacement::Replacement;
use crate::ui;

use std::boxed::Box;
use std::path::{Path, PathBuf};

use env_logger::Builder;

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Gui {}

impl Gui {
    pub fn new() -> Self {
        Gui {}
    }
}

impl ui::Interface for Gui {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        logger_builder.try_init()
    }

    fn confirm(&self, _replacement: &Replacement) -> Confirmation {
        Confirmation::Abort
    }

    fn process(
        &mut self,
        matchers: &Vec<Box<dyn Matcher>>,
        paths: &Vec<PathBuf>,
    ) {
        function(Box::new(DirectCommunication::new(self)), matchers, paths);
    }
}
