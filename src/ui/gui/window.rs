use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::replacement::Replacement;
use crate::ui::gui::processing;

use std::path::PathBuf;

use iced::executor;
use iced::{Application, Color, Command, Element, Length, Subscription, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Processing(processing::Event),
}

pub struct Window {
    matchers: Vec<Box<dyn Matcher>>,
    paths: Vec<PathBuf>,
    state: State,
    progress: Progress,
}

#[derive(Default)]
enum State {
    #[default]
    Booting,
    Processing(processing::Connection),
    Finished,
}

#[derive(Default)]
struct Progress {
    index: usize,
    current: Option<PathBuf>,
    log: Vec<ProcessingResult>,
}

#[derive(Clone)]
enum ProcessingResult {
    Success(Replacement),
    Error(PathBuf, String)
}

impl std::fmt::Display for ProcessingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Success(rep) =>  write!(f, "{}", rep),
            Self::Error(path, error) =>  write!(f, "Error replacing {:?}: {:}", path, error)
        }
    }
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
                state: State::default(),
                progress: Progress::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Prefix by date")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        use processing::Event;
        log::debug!("Message: {:?}", message);

        match message {
            Message::Processing(event) => {
                match event {
                    Event::Ready(connection) => {
                        self.state = State::Processing(connection);
                    }
                    Event::Processing(path) => {
                        self.progress.current = Some(path);
                    }
                    Event::ProcessingOk(rep) => {
                        self.progress.index += 1;
                        self.progress.log.push(ProcessingResult::Success(rep));
                    }
                    Event::ProcessingErr(path, error) => {
                        self.progress.index += 1;
                        self.progress.log.push(ProcessingResult::Error(path, error));
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
        use iced::widget::{column, container, progress_bar, scrollable, text, Column};

        let name = match &self.progress.current {
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

        column![
            progress_bar(
                0.0..=(self.paths.len() as f32),
                self.progress.index as f32
            ),
            message,
            scrollable(
                Column::with_children(
                    self.progress.log
                    .iter().cloned().map(text).map(Element::from).collect()
                )
                .width(Length::Fill)
            )
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .spacing(10)
        .into()
    }
}
