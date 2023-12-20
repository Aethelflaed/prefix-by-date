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
    Keyboard(iced::keyboard::Event),
    Confirm(Confirmation),
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
    current: Current,
    log: Vec<ProcessingResult>,
}

#[derive(Default)]
enum Current {
    #[default]
    None,
    Path(PathBuf),
    Replacement(Replacement),
}

#[derive(Clone)]
enum ProcessingResult {
    Success(Replacement),
    Error(PathBuf, String),
}

impl std::fmt::Display for ProcessingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Success(rep) => write!(f, "{}", rep),
            Self::Error(path, error) => {
                write!(f, "Error replacing {:?}: {:}", path, error)
            }
        }
    }
}

impl Window {
    fn confirm(&mut self, conf: Confirmation) {
        if matches!(self.progress.current, Current::Replacement(_)) {
            if let State::Processing(connection) = &mut self.state {
                connection.send(conf);
            }
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

        match message {
            Message::Processing(event) => {
                log::debug!("Processing event: {:?}", event);
                match event {
                    Event::Ready(connection) => {
                        self.state = State::Processing(connection);

                        Command::none()
                    }
                    Event::Processing(path) => {
                        self.progress.current = Current::Path(path);

                        Command::none()
                    }
                    Event::ProcessingOk(rep) => {
                        self.progress.index += 1;
                        self.progress.log.push(ProcessingResult::Success(rep));

                        Command::none()
                    }
                    Event::ProcessingErr(path, error) => {
                        self.progress.index += 1;
                        self.progress
                            .log
                            .push(ProcessingResult::Error(path, error));

                        Command::none()
                    }
                    Event::Confirm(rep) => {
                        self.progress.current = Current::Replacement(rep);

                        Command::none()
                    }
                    Event::Finished | Event::Aborted => {
                        self.state = State::Finished;

                        iced::window::close()
                    }
                }
            }
            Message::Confirm(confirmation) => {
                self.confirm(confirmation);
                Command::none()
            }
            Message::Keyboard(event) => match event {
                iced::keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                } => {
                    if modifiers.control()
                        && key_code == iced::keyboard::KeyCode::Q
                    {
                        iced::window::close()
                    } else if modifiers.is_empty() {
                        match key_code {
                            iced::keyboard::KeyCode::Y => {
                                self.confirm(Confirmation::Accept)
                            }
                            _ => {}
                        };

                        Command::none()
                    } else {
                        Command::none()
                    }
                }
                _ => Command::none(),
            },
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            processing::connect(&self.matchers, &self.paths)
                .map(Message::Processing),
            iced::subscription::events_with(|event, _status| match event {
                iced::Event::Keyboard(kevent) => {
                    Some(Message::Keyboard(kevent))
                }
                _ => None,
            }),
        ])
    }

    fn view(&self) -> Element<Message> {
        use iced::alignment;
        use iced::widget::{
            button, column, container, progress_bar, row, scrollable, text,
            Column,
        };

        let message = match &self.progress.current {
            Current::None => String::from("Booting..."),
            Current::Path(path) => format!("{:?}", path),
            Current::Replacement(rep) => format!("{:}", rep),
        };

        let message: Element<_> =
            container(text(message).style(Color::from_rgb8(0x88, 0x88, 0x88)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into();

        let buttons = row![
            button(
                text("Accept")
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
            )
            .on_press(Message::Confirm(Confirmation::Accept)),
            button(
                text("Always")
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
            )
            .on_press(Message::Confirm(Confirmation::Always))
        ];

        column![
            progress_bar(
                0.0..=(self.paths.len() as f32),
                self.progress.index as f32
            ),
            message,
            buttons,
            scrollable(
                Column::with_children(
                    self.progress
                        .log
                        .iter()
                        .cloned()
                        .map(text)
                        .map(Element::from)
                        .collect()
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
