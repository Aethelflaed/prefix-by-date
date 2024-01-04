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
    Keyboard(iced::keyboard::Event, iced::event::Status),
    Confirm(Confirmation),
    ToggleLogs,
    SetCustomize(String),
    CustomizeInput(String),
    CustomizeSubmit,
}

pub struct Window {
    matchers: Vec<Box<dyn Matcher>>,
    paths: Vec<PathBuf>,
    state: State,
    progress: Progress,
    log: bool,
    debug: bool,
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
    logs: Vec<ProcessingResult>,
}

#[derive(Default)]
enum Current {
    #[default]
    None,
    Path(PathBuf),
    Confirm(Change),
    Rescue(Change),
}

struct Change {
    replacement: Replacement,
    alternatives: HashMap<String, Replacement>,
    customize: Option<String>,
}

impl Change {
    fn customize_button(&self) -> Option<Element<'_, Message>> {
        if self.customize.is_none() || !self.alternatives.is_empty() {
            Some(
                simple_button(
                    "Custom",
                    Message::SetCustomize(
                        self.replacement.new_file_stem.clone(),
                    ),
                )
                .into(),
            )
        } else {
            None
        }
    }

    fn new_confirm(
        replacement: Replacement,
        matchers: &[Box<dyn Matcher>],
    ) -> Self {
        let path_buf = replacement.path();
        let path = path_buf.as_path();

        Self {
            alternatives: matchers
                .iter()
                .filter_map(|matcher| {
                    matcher.check(path).and_then(|rep| {
                        // Skip alternatives similar to the replacement
                        if rep.new_file_stem == replacement.new_file_stem {
                            None
                        } else {
                            Some((matcher.name().to_string(), rep))
                        }
                    })
                })
                .collect(),
            replacement,
            customize: None,
        }
    }

    fn new_current_confirm(
        replacement: Replacement,
        matchers: &[Box<dyn Matcher>],
    ) -> Current {
        Current::Confirm(Self::new_confirm(replacement, matchers))
    }

    fn new_rescue(replacement: Replacement) -> Self {
        Self {
            replacement,
            alternatives: Default::default(),
            customize: None,
        }
    }

    fn new_current_rescue(replacement: Replacement) -> Current {
        Current::Rescue(Self::new_rescue(replacement))
    }
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
                    | Confirmation::Skip
                    | Confirmation::Refuse => connection.send(conf),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn toggle_log(&mut self) {
        self.log = !self.log;
    }
    fn toggle_debug(&mut self) {
        self.debug = !self.debug;
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
                log: false,
                debug: false,
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
                        self.progress.current =
                            Change::new_current_confirm(rep, &self.matchers);

                        Command::none()
                    }
                    Event::Rescue(rep) => {
                        self.progress.current = Change::new_current_rescue(rep);

                        Command::none()
                    }
                    Event::Finished | Event::Aborted => {
                        self.state = State::Finished;

                        iced::window::close()
                    }
                }
            }
            Message::ToggleLogs => {
                self.toggle_log();

                Command::none()
            }
            Message::SetCustomize(string) => {
                match &mut self.progress.current {
                    Current::Confirm(change) | Current::Rescue(change) => {
                        change.customize = Some(string)
                    }
                    _ => {}
                };

                Command::none()
            }
            Message::CustomizeInput(string) => {
                match &mut self.progress.current {
                    Current::Confirm(change) | Current::Rescue(change) => {
                        change.customize = Some(string)
                    }
                    _ => {}
                };

                Command::none()
            }
            Message::CustomizeSubmit => {
                match &self.progress.current {
                    Current::Confirm(change) | Current::Rescue(change) => {
                        if let Some(value) = change.customize.clone() {
                            let mut rep = change.replacement.clone();
                            rep.new_file_stem = value;
                            self.confirm(Confirmation::Replace(rep));
                        }
                    }
                    _ => {}
                }

                Command::none()
            }
            Message::Confirm(confirmation) => {
                self.confirm(confirmation);
                Command::none()
            }
            Message::Keyboard(event, status) => match event {
                iced::keyboard::Event::KeyPressed {
                    key_code,
                    modifiers,
                } => {
                    if modifiers.control() && key_code == KeyCode::Q {
                        iced::window::close()
                    } else if modifiers.is_empty()
                        && status == iced::event::Status::Ignored
                    {
                        match key_code {
                            KeyCode::Y => self.confirm(Confirmation::Accept),
                            KeyCode::A => self.confirm(Confirmation::Always),
                            KeyCode::S => self.confirm(Confirmation::Skip),
                            KeyCode::R => self.confirm(Confirmation::Refuse),
                            KeyCode::I => self.confirm(Confirmation::Ignore),
                            KeyCode::Q => self.confirm(Confirmation::Abort),
                            KeyCode::L => self.toggle_log(),
                            KeyCode::D => self.toggle_debug(),
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
            iced::subscription::events_with(|event, status| match event {
                iced::Event::Keyboard(kevent) => {
                    Some(Message::Keyboard(kevent, status))
                }
                _ => None,
            }),
        ])
    }

    fn view(&self) -> Element<Message> {
        use iced::alignment::Alignment;
        use iced::widget::{column, container, progress_bar, row, text, Row};

        let message: Element<_> = match &self.progress.current {
            Current::None => text("Booting").into(),
            Current::Path(path) => {
                text(format!("Processing {}", path.display())).into()
            }
            Current::Confirm(change) => {
                let rep = &change.replacement;

                column![
                    text(format!("In {}", rep.parent.display())).size(12),
                    text(format!("Replace {:?} with:", rep.file_name(),)),
                    text(rep.new_file_name()),
                ]
                .into()
            }
            Current::Rescue(change) => {
                let rep = &change.replacement;

                column![
                    text(format!("In {}", rep.parent.display())).size(12),
                    text(format!(
                        "No match was found for {:?}",
                        rep.file_name()
                    )),
                ]
                .into()
            }
        };

        let mut buttons = Row::with_children(match &self.progress.current {
            Current::None | Current::Path(_) => vec![],
            Current::Confirm(change) => {
                let mut buttons = vec![
                    conf_button("Yes", Confirmation::Accept).into(),
                    conf_button("Always", Confirmation::Always).into(),
                ];

                if let Some(button) = change.customize_button() {
                    buttons.push(button);
                }
                buttons.push(conf_button("Skip", Confirmation::Skip).into());
                buttons
                    .push(conf_button("Refuse", Confirmation::Refuse).into());
                buttons
                    .push(conf_button("Ignore", Confirmation::Ignore).into());
                buttons.push(conf_button("Quit", Confirmation::Abort).into());
                buttons
            }
            Current::Rescue(change) => {
                let mut buttons = vec![];

                if let Some(button) = change.customize_button() {
                    buttons.push(button);
                }
                buttons.push(conf_button("Skip", Confirmation::Skip).into());
                buttons.push(conf_button("Quit", Confirmation::Abort).into());
                buttons
            }
        })
        .spacing(10);

        buttons = buttons.push(simple_button("Logs", Message::ToggleLogs));

        let buttons =
            container(buttons).width(Length::Fill).center_x().center_y();

        let mut content = column![message, buttons,]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .spacing(10);

        if let Current::Confirm(change) = &self.progress.current {
            if !change.alternatives.is_empty() {
                content = content.push(text("Or choose from an alternatives"));
                content = content.push(
                    container(
                        column(
                            change
                                .alternatives
                                .values()
                                .map(|rep| {
                                    row![
                                        conf_button(
                                            "Use",
                                            Confirmation::Replace(rep.clone())
                                        ),
                                        simple_button(
                                            "Customize",
                                            Message::SetCustomize(
                                                rep.new_file_stem.clone()
                                            )
                                        ),
                                        text(rep.new_file_name()),
                                    ]
                                    .align_items(Alignment::Center)
                                    .spacing(10)
                                })
                                .map(Element::from)
                                .collect(),
                        )
                        .spacing(10),
                    )
                    .width(Length::Fill),
                );
            }
        }

        match &self.progress.current {
            Current::Confirm(change) | Current::Rescue(change) => {
                if let Some(string) = &change.customize {
                    content = content.push(customize(string));
                }
            }
            _ => {}
        }

        content = content.push(progress_bar(
            0.0..=(self.paths.len() as f32),
            self.progress.index as f32,
        ));

        if self.log {
            content = content.push(scrollable_logs(&self.progress.logs));
        }

        let mut content: Element<_> = content.into();

        if self.debug {
            content = content.explain(Color::BLACK);
        }

        content
    }

    fn theme(&self) -> Theme {
        Theme::Dark
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

fn customize(string: &str) -> Element<'_, Message> {
    use iced::widget::TextInput;

    TextInput::new("Type the new file name here", string)
        .on_input(Message::CustomizeInput)
        .on_submit(Message::CustomizeSubmit)
        .padding(10)
        .into()
}
