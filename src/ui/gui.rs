#![cfg(feature = "gui")]

use crate::application::Result;
use crate::matcher::Matcher;
use crate::ui;

use std::path::PathBuf;

mod processing;
mod window;

pub struct Gui {}

impl Gui {
    pub fn new() -> Self {
        Gui {}
    }
}

impl ui::Interface for Gui {
    fn process(
        &mut self,
        matchers: &[Box<dyn Matcher>],
        paths: &[PathBuf],
    ) -> Result<()> {
        use iced::{Application, Settings};

        window::Window::run(Settings {
            flags: (matchers.to_owned(), paths.to_owned()),
            window: iced::window::Settings {
                size: (500, 500),
                ..Default::default()
            },
            ..Settings::default()
        })
        .expect("Window to start");
        Ok(())
    }
}
