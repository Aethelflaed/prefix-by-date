#![cfg(feature = "gui")]

use crate::application::Result;
use crate::matcher::Matcher;
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

    fn process(
        &mut self,
        matchers: &[Box<dyn Matcher>],
        paths: &[PathBuf],
    ) -> Result<()> {
        use iced::{Application, Settings};

        window::Window::run(Settings {
            flags: (matchers.to_owned(), paths.to_owned()),
            window: iced::window::Settings {
                size: (300, 300),
                ..Default::default()
            },
            ..Settings::default()
        })
        .expect("Window to start");
        Ok(())
    }
}
