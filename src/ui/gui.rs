#![cfg(feature = "gui")]

use crate::application::Result;
use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::replacement::Replacement;
use crate::ui;

use std::path::PathBuf;

use env_logger::Builder;

type LogResult = std::result::Result<(), log::SetLoggerError>;

mod processing;
mod window;

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
        matchers: &[Box<dyn Matcher>],
        paths: &[PathBuf],
    ) -> Result<()> {
        use iced::{Application, Settings};

        window::Window::run(Settings {
            flags: (matchers.to_owned(), paths.to_owned()),
            ..Settings::default()
        })
        .unwrap(); // XXX
        Ok(())
    }
}
