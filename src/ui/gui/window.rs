use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::ui::actions::Action;
use crate::ui::gui::processing;
use crate::ui::state::{Current, ProcessingResult, State};

use std::path::PathBuf;

use iced::executor;
use iced::keyboard::KeyCode;
use iced::{Application, Color, Command, Element, Length, Subscription, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Idle,
    Processing(processing::Event),
    Action(Action),
    ToggleLog,
    ToggleDebug,
    Quit,
    MaybeShortcut(KeyCode),
}

pub struct Window {
    matchers: Vec<Box<dyn Matcher>>,
    paths: Vec<PathBuf>,
    processing_state: ProcessingState,
    state: State,
    log: bool,
    debug: bool,
}

#[derive(Default)]
enum ProcessingState {
    #[default]
    Booting,
    Processing(processing::Connection),
    Finished,
}

impl Window {
    fn execute(&mut self, action: Action) -> Command<Message> {
        match action {
            Action::Customize(file_stem) => {
                self.state.customize(file_stem);

                focus_on(CUSTOMIZE_INPUT_ID.clone())
            }
            Action::ConfirmCustomization => {
                if let Some(rep) = self.state.customized_replacement() {
                    self.send_confirmation(Confirmation::Replace(rep))
                } else {
                    Command::none()
                }
            }
            _ => self.send_confirmation(
                action.try_into().expect("Customize handled locally"),
            ),
        }
    }

    fn send_confirmation(&mut self, conf: Confirmation) -> Command<Message> {
        use ProcessingState::Processing;

        if !self.state.set_current_resolving(conf.clone()) {
            return Command::none();
        }

        if let Processing(connection) = &mut self.processing_state {
            let mut connection = connection.clone();

            return Command::perform(
                async move {
                    connection.send_async(conf).await;
                },
                |_| Message::Idle,
            );
        }

        Command::none()
    }

    fn update_processing_event(
        &mut self,
        event: processing::Event,
    ) -> Command<Message> {
        use processing::Event::*;

        log::debug!("Processing event: {:?}", event);

        match event {
            Ready(connection) => {
                self.processing_state = ProcessingState::Processing(connection);
            }
            Processing(path) => {
                self.state.set_current_path(path);
            }
            ProcessingOk(rep) => {
                self.state.set_current_success(rep);
            }
            ProcessingErr(path, error) => {
                self.state.set_current_failure(path, error);
            }
            Confirm(rep) => {
                self.state.set_current_confirm(rep, &self.matchers);
            }
            Rescue(rep) => {
                self.state.set_current_rescue(rep);
            }
            Finished | Aborted => {
                self.processing_state = ProcessingState::Finished;

                return iced::window::close();
            }
        }

        Command::none()
    }
}

impl Application for Window {
    type Message = Message;
    type Theme = Theme;
    type Flags = (Vec<Box<dyn Matcher>>, Vec<PathBuf>);
    type Executor = executor::Default;

    fn new((matchers, paths): Self::Flags) -> (Self, Command<Message>) {
        let len = paths.len();
        (
            Window {
                matchers,
                paths,
                processing_state: ProcessingState::default(),
                state: State::new(len),
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
        match message {
            Message::Idle => Command::none(),
            Message::Processing(event) => self.update_processing_event(event),
            Message::ToggleLog => {
                self.log = !self.log;

                Command::none()
            }
            Message::ToggleDebug => {
                self.debug = !self.debug;

                Command::none()
            }
            Message::Action(action) => self.execute(action),
            Message::Quit => iced::window::close(),
            Message::MaybeShortcut(key_code) => {
                let predicate = |action: &&Action| {
                    if let Some(code) = iced_shortcut_for(action) {
                        key_code == code
                    } else {
                        false
                    }
                };

                if let Some(action) =
                    self.state.actions().iter().find(predicate).cloned()
                {
                    self.execute(action)
                } else {
                    Command::none()
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            processing::connect(&self.matchers, &self.paths)
                .map(Message::Processing),
            iced::subscription::events_with(|event, status| {
                filter_events(event, status)
            }),
        ])
    }

    fn view(&self) -> Element<Message> {
        use iced::alignment::Alignment;
        use iced::widget::{column, container, progress_bar, row, text, Row};

        let message: Element<_> = match &self.state.current() {
            Current::None => text("Booting").into(),
            Current::Path(path) => {
                text(format!("Processing {}", path.display())).into()
            }
            Current::Confirm(change) => {
                let rep = &change.replacement;

                column![
                    text(format!("In {}", rep.parent.display())).size(12),
                    text(format!("Replace {} with:", rep.file_name())),
                    text(rep.new_file_name()),
                ]
                .into()
            }
            Current::Rescue(change) => {
                let rep = &change.replacement;

                column![
                    text(format!("In {}", rep.parent.display())).size(12),
                    text(format!("No match was found for {}", rep.file_name())),
                ]
                .into()
            }
            _ => text("Processing...").into(),
        };

        let mut buttons = Row::with_children(
            self.state
                .actions()
                .iter()
                .cloned()
                .filter_map(|action| match action {
                    Action::Replace(_) => None,
                    _ => Some(action_button(action).into()),
                })
                .collect::<Vec<Element<_>>>(),
        )
        .spacing(10);

        buttons = buttons.push(simple_button("Logs", Message::ToggleLog));

        let buttons =
            container(buttons).width(Length::Fill).center_x().center_y();

        let mut content = column![message, buttons,]
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .spacing(10);

        if let Current::Confirm(change) = &self.state.current() {
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
                                        action_button(Action::Replace(
                                            rep.clone()
                                        )),
                                        action_button(Action::Customize(
                                            rep.new_file_stem.clone()
                                        )),
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

        match &self.state.current() {
            Current::Confirm(change) | Current::Rescue(change) => {
                if let Some(string) = &change.customize {
                    content = content.push(customize(string));
                }
            }
            _ => {}
        }

        content = content.push(progress_bar(
            0.0..=(self.state.len() as f32),
            self.state.index() as f32,
        ));

        if self.log {
            content = content.push(scrollable_logs(self.state.logs()));
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

use once_cell::sync::Lazy;
static CUSTOMIZE_INPUT_ID: Lazy<iced::widget::text_input::Id> =
    Lazy::new(iced::widget::text_input::Id::unique);

fn focus_on<T>(id: T) -> Command<Message>
where
    T: Into<iced::advanced::widget::Id>,
{
    Command::widget(iced::advanced::widget::operation::focusable::focus(
        id.into(),
    ))
}

fn filter_events(
    event: iced::event::Event,
    status: iced::event::Status,
) -> Option<Message> {
    use iced::{
        event::Status::Ignored, keyboard::Event::KeyPressed, Event::Keyboard,
    };

    if let Keyboard(KeyPressed {
        key_code,
        modifiers,
    }) = event
    {
        // Whatever the context, ctrl-Q quits the app
        if modifiers.control() && key_code == KeyCode::Q {
            return Some(Message::Quit);
        }

        // Keyboard shortcuts
        if status == Ignored && modifiers.is_empty() {
            match key_code {
                KeyCode::L => return Some(Message::ToggleLog),
                KeyCode::D => return Some(Message::ToggleDebug),
                _ => {}
            }

            if Action::all()
                .iter()
                .any(|action| iced_shortcut_for(action) == Some(key_code))
            {
                return Some(Message::MaybeShortcut(key_code));
            }
        }
    }

    None
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

fn customize(string: &str) -> Element<'_, Message> {
    use iced::widget::TextInput;

    TextInput::new("Type the new file name here", string)
        .id(CUSTOMIZE_INPUT_ID.clone())
        .on_input(|value| Message::Action(Action::Customize(value)))
        .on_submit(Message::Action(Action::ConfirmCustomization))
        .padding(10)
        .into()
}

fn iced_shortcut_for(action: &Action) -> Option<KeyCode> {
    match action {
        Action::Accept => Some(KeyCode::Y),
        Action::Always => Some(KeyCode::A),
        Action::Customize(_) => Some(KeyCode::C),
        Action::ViewAlternatives => None,
        Action::Replace(_) => None,
        Action::Skip => Some(KeyCode::S),
        Action::Refuse => Some(KeyCode::R),
        Action::Ignore => Some(KeyCode::I),
        Action::Abort => Some(KeyCode::Q),
        Action::Cancel => None,
        Action::ConfirmCustomization => None,
    }
}

fn action_button(action: Action) -> iced::widget::Button<'static, Message> {
    let label = match action {
        Action::Accept => "Yes",
        Action::Always => "Always",
        Action::Customize(_) => "Custom",
        Action::Skip => "Skip",
        Action::Refuse => "Refuse",
        Action::Ignore => "Ignore",
        Action::Abort => "Quit",
        Action::Replace(_) => "Use",
        Action::ConfirmCustomization => "Confirm",
        Action::ViewAlternatives => "Alternatives",
        Action::Cancel => "Cancel",
    };

    simple_button(label, Message::Action(action))
}
