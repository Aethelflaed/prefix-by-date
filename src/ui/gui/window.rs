use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::replacement::Replacement;
use crate::ui::gui::processing;

use std::collections::HashMap;
use std::path::PathBuf;

use iced::executor;
use iced::keyboard::KeyCode;
use iced::{Application, Color, Command, Element, Length, Subscription, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Processing(processing::Event),
    Keyboard(iced::keyboard::Event),
    Confirm(Confirmation),
    ToggleLogs,
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
    log: bool,
    debug: bool,
    replacements: HashMap<String, Replacement>,
    logs: Vec<ProcessingResult>,
}

#[derive(Default)]
enum Current {
    #[default]
    None,
    Path(PathBuf),
    Confirm(Replacement),
    Rescue(Replacement),
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
            Self::Error(_path, error) => write!(f, "{}", error),
        }
    }
}

impl Window {
    fn confirm(&mut self, conf: Confirmation) {
        if let State::Processing(connection) = &mut self.state {
            match self.progress.current {
                Current::Confirm(_) => {
                    connection.send(conf);
                }
                Current::Rescue(_) => match conf {
                    Confirmation::Replace(_)
                    | Confirmation::Abort
                    | Confirmation::Refuse => connection.send(conf),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn toggle_logs(&mut self) {
        self.progress.log = !self.progress.log;
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
                        self.progress.replacements.clear();
                        self.progress.current = Current::Path(path);

                        Command::none()
                    }
                    Event::ProcessingOk(rep) => {
                        self.progress.index += 1;
                        self.progress.logs.push(ProcessingResult::Success(rep));

                        Command::none()
                    }
                    Event::ProcessingErr(path, error) => {
                        self.progress.index += 1;
                        self.progress
                            .logs
                            .push(ProcessingResult::Error(path, error));

                        Command::none()
                    }
                    Event::Confirm(rep) => {
                        for matcher in &self.matchers {
                            if let Some(replacement) =
                                matcher.check(rep.path().as_path())
                            {
                                self.progress.replacements.insert(
                                    matcher.name().to_string(),
                                    replacement,
                                );
                            }
                        }

                        self.progress.current = Current::Confirm(rep);

                        Command::none()
                    }
                    Event::Rescue(rep) => {
                        self.progress.current = Current::Rescue(rep);

                        Command::none()
                    }
                    Event::Finished | Event::Aborted => {
                        self.state = State::Finished;

                        iced::window::close()
                    }
                }
            }
            Message::ToggleLogs => {
                self.toggle_logs();

                Command::none()
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
                    if modifiers.control() && key_code == KeyCode::Q {
                        iced::window::close()
                    } else if modifiers.is_empty() {
                        match key_code {
                            KeyCode::Y => self.confirm(Confirmation::Accept),
                            KeyCode::A => self.confirm(Confirmation::Always),
                            KeyCode::S => self.confirm(Confirmation::Skip),
                            KeyCode::R => self.confirm(Confirmation::Refuse),
                            KeyCode::I => self.confirm(Confirmation::Ignore),
                            KeyCode::Q => self.confirm(Confirmation::Abort),
                            KeyCode::L => self.toggle_logs(),
                            KeyCode::D => {
                                self.progress.debug = !self.progress.debug;
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
        use iced::alignment::Alignment;
        use iced::widget::{
            column, container, progress_bar, row, text, Container, Row,
        };

        let message = match &self.progress.current {
            Current::None => String::from("Booting..."),
            Current::Path(path) => format!("{:?}", path),
            Current::Confirm(rep) => format!("{:}", rep),
            Current::Rescue(rep) => format!("{:}", rep),
        };

        let message: Element<_> =
            container(text(message).style(Color::from_rgb8(0x88, 0x88, 0x88)))
                .width(Length::Fill)
                .center_x()
                .center_y()
                .into();

        let mut buttons = Row::with_children(vec![]).spacing(10);
        match self.progress.current {
            Current::None | Current::Path(_) => {}
            Current::Confirm(_) => {
                buttons =
                    buttons.push(conf_button("Yes", Confirmation::Accept));
                buttons =
                    buttons.push(conf_button("Always", Confirmation::Always));
                buttons = buttons.push(conf_button("Skip", Confirmation::Skip));
                buttons =
                    buttons.push(conf_button("Refuse", Confirmation::Refuse));
                buttons =
                    buttons.push(conf_button("Ignore", Confirmation::Ignore));
                buttons =
                    buttons.push(conf_button("Quit", Confirmation::Abort));
            }
            Current::Rescue(_) => {}
        }

        buttons = buttons.push(simple_button("Logs", Message::ToggleLogs));
        let buttons =
            container(buttons).width(Length::Fill).center_x().center_y();

        let alternatives: Container<'_, Message> = container(
            column(
                self.progress
                    .replacements
                    .iter()
                    .map(|(name, rep)| {
                        row![
                            conf_button(
                                name,
                                Confirmation::Replace(rep.clone())
                            ),
                            text(format!("{}", rep)),
                        ]
                        .align_items(Alignment::Center)
                        .spacing(10)
                    })
                    .map(Element::from)
                    .collect(),
            )
            .spacing(10),
        )
        .width(Length::Fill);

        let mut content = column![
            message,
            buttons,
            alternatives,
            progress_bar(
                0.0..=(self.paths.len() as f32),
                self.progress.index as f32
            ),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .spacing(10);

        if self.progress.log {
            content = content.push(scrollable_logs(&self.progress.logs));
        }

        let mut content: Element<_> = content.into();

        if self.progress.debug {
            content = content.explain(Color::BLACK);
        }

        content
    }
}

fn scrollable_logs(
    logs: &[ProcessingResult],
) -> iced::widget::Scrollable<'_, Message> {
    use iced::widget::{column, scrollable, text};
    scrollable(
        column(
            logs.iter()
                .rev()
                .cloned()
                .map(text)
                .map(Element::from)
                .collect(),
        )
        .width(Length::Fill),
    )
}

fn simple_button(
    label: &str,
    message: Message,
) -> iced::widget::Button<'_, Message> {
    use iced::{
        alignment,
        widget::{button, text},
    };
    button(
        text(label)
            .width(Length::Fill)
            .horizontal_alignment(alignment::Horizontal::Center),
    )
    .on_press(message)
}

fn conf_button(
    label: &str,
    confirmation: Confirmation,
) -> iced::widget::Button<'_, Message> {
    simple_button(label, Message::Confirm(confirmation))
}
