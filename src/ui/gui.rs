#![cfg(feature = "gui")]

use crate::application::Result;
use crate::matcher::Matcher;
use crate::ui;

use std::path::PathBuf;

mod processing;
mod window;

use window::Window;

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
        let matchers = matchers.to_owned();
        let paths = paths.to_owned();

        iced::application(Window::title, Window::update, Window::view)
            .window_size((500., 500.))
            .subscription(Window::subscription)
            .theme(Window::theme)
            .run_with(|| Window::new(matchers, paths))
            .expect("Window to start");
        Ok(())
    }
}
