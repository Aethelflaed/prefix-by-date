use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::ui::gui::processing;

use std::path::PathBuf;

use iced::executor;
use iced::widget::{column, container, text};
use iced::{
    Application, Color, Command, Element, Length, Subscription, Theme,
};

#[derive(Debug, Clone)]
pub enum Message {
    Processing(processing::Event),
}

pub struct Window {
    matchers: Vec<Box<dyn Matcher>>,
    paths: Vec<PathBuf>,
    state: State,
    processing: Option<PathBuf>,
}

#[derive(Default)]
enum State {
    #[default]
    Booting,
    Processing(processing::Connection),
    Finished,
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
                state: Default::default(),
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
            Message::Processing(event) => {
                match event {
                    Event::Ready(connection) => {
                        self.state = State::Processing(connection);
                    }
                    Event::Processing(path) => {
                        self.processing = Some(path);
                    }
                    Event::ProcessingOk(_) => {
                        //self.processing = None;
                    }
                    Event::ProcessingErr(_, _) => {
                        //self.processing = None;
                    }
                    Event::Confirm(_) => {
                        if let State::Processing(connection) = &mut self.state {
                            connection.send(Confirmation::Abort);
                        }
                    }
                    Event::Finished | Event::Aborted => {
                        self.state = State::Finished;
                    }
                };
                Command::none()
            }
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
