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

use iced::executor;
use iced::widget::{column, container, text};
use iced::{
    Application, Color, Command, Element, Length, Settings, Subscription, Theme,
};

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
        Window::run(Settings {
            flags: (matchers.to_owned(), paths.to_owned()),
            ..Settings::default()
        })
        .unwrap(); // XXX
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum Message {
    Processing(processing::Event),
}

struct Window {
    matchers: Vec<Box<dyn Matcher>>,
    paths: Vec<PathBuf>,
    confirmation_sender: Option<std::sync::mpsc::Sender<Confirmation>>,
    processing: Option<PathBuf>,
}

impl Application for Window {
    type Message = Message;
    type Theme = Theme;
    type Flags = (Vec<Box<dyn Matcher>>, Vec<PathBuf>);
    type Executor = executor::Default;

    fn new((matchers, paths): Self::Flags) -> (Self, Command<Message>) {
        (
            Window {
                matchers,
                paths,
                confirmation_sender: None,
                processing: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Prefix by date")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        use processing::Event;
        log::error!("Message: {:?}", message);

        match message {
            Message::Processing(event) => match event {
                Event::Ready(confirmation_sender) => {
                    self.confirmation_sender = Some(confirmation_sender);

                    Command::none()
                }
                Event::Processing(path) => {
                    self.processing = Some(path);

                    Command::none()
                }
                Event::ProcessingOk(_) => {
                    //self.processing = None;
                    Command::none()
                }
                Event::ProcessingErr(_, _) => {
                    //self.processing = None;
                    Command::none()
                }
                Event::Confirm(_) => {
                    if let Some(sender) = &self.confirmation_sender {
                        let _ = sender.send(Confirmation::Abort);
                    }
                    Command::none()
                }
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        processing::connect(&self.matchers, &self.paths)
            .map(Message::Processing)
    }

    fn view(&self) -> Element<Message> {
        let name = match &self.processing {
            Some(path) => format!("{:?}", path),
            None => String::from("..."),
        };

        let message: Element<_> =
            container(text(name).style(Color::from_rgb8(0x88, 0x88, 0x88)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into();

        column![message]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .spacing(10)
            .into()
    }
}
