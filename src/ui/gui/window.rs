use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::ui::actions::Action;
use crate::ui::gui::processing;
use crate::ui::state::{Current, ProcessingResult, State};

use std::path::PathBuf;

use iced::keyboard::{Key, Modifiers};
use iced::{Color, Element, Length, Subscription, Task, Theme};

#[derive(Debug, Clone)]
pub enum Message {
    Idle,
    Processing(processing::Event),
    Action(Action),
    ToggleLog,
    ToggleDebug,
    Quit,
    MaybeShortcut(Key<&'static str>),
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
    fn execute(&mut self, action: Action) -> Task<Message> {
        use Action::*;

        match action {
            Customize(file_stem) => {
                self.state.customize(file_stem);

                iced::widget::text_input::focus(CUSTOMIZE_INPUT_ID.clone())
            }
            ConfirmCustomization => {
                if let Some(rep) = self.state.customized_replacement() {
                    self.send_confirmation(Confirmation::Replace(rep))
                } else {
                    Task::none()
                }
            }
            Accept => self.send_confirmation(Confirmation::Accept),
            Always => self.send_confirmation(Confirmation::Always),
            Skip => self.send_confirmation(Confirmation::Skip),
            Refuse => self.send_confirmation(Confirmation::Refuse),
            Ignore => self.send_confirmation(Confirmation::Ignore),
            Abort => self.send_confirmation(Confirmation::Abort),
            Replace(rep) => self.send_confirmation(Confirmation::Replace(rep)),
            ViewAlternatives => unimplemented!(),
            Cancel => unimplemented!(),
        }
    }

    fn send_confirmation(&mut self, conf: Confirmation) -> Task<Message> {
        use ProcessingState::Processing;

        if !self.state.set_current_resolving(conf.clone()) {
            return Task::none();
        }

        if let Processing(connection) = &mut self.processing_state {
            let mut connection = connection.clone();

            return Task::perform(
                async move {
                    connection.send_async(conf).await;
                },
                |_| Message::Idle,
            );
        }

        Task::none()
    }

    fn update_processing_event(
        &mut self,
        event: processing::Event,
    ) -> Task<Message> {
        use processing::Event::*;

        log::debug!("Processing event: {:?}", event);

        match event {
            Initialization(mut connection) => {
                let matchers = self.matchers.clone();
                let paths = self.paths.clone();

                use processing::InitializationData::*;

                return Task::perform(
                    async move {
                        connection.send_async(Matchers(matchers)).await;
                        connection.send_async(Paths(paths)).await;
                        connection.send_async(Done).await;
                    },
                    |_| Message::Idle,
                );
            }
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

                return iced::window::get_latest()
                    .and_then(iced::window::close);
            }
        }

        Task::none()
    }

    pub fn new(
        matchers: Vec<Box<dyn Matcher>>,
        paths: Vec<PathBuf>,
    ) -> (Self, Task<Message>) {
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
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        String::from("Prefix by date")
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Idle => Task::none(),
            Message::Processing(event) => self.update_processing_event(event),
            Message::ToggleLog => {
                self.log = !self.log;

                Task::none()
            }
            Message::ToggleDebug => {
                self.debug = !self.debug;

                Task::none()
            }
            Message::Action(action) => self.execute(action),
            Message::Quit => {
                iced::window::get_latest().and_then(iced::window::close)
            }
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
                    Task::none()
                }
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            Subscription::run(processing::connect).map(Message::Processing),
            iced::keyboard::on_key_press(handle_hotkey),
        ])
    }

    pub fn view(&self) -> Element<'_, Message> {
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
            self.state.actions().iter().cloned().filter_map(|action| {
                match action {
                    Action::Replace(_)
                    | Action::ViewAlternatives
                    | Action::Cancel => None,
                    _ => Some(action_button(action).into()),
                }
            }),
        )
        .spacing(10);

        buttons = buttons.push(simple_button("Logs", Message::ToggleLog));

        let mut content = column![message, buttons,]
            .width(Length::Fill)
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
                                    .spacing(10)
                                })
                                .map(Element::from),
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

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}

use once_cell::sync::Lazy;
static CUSTOMIZE_INPUT_ID: Lazy<iced::widget::text_input::Id> =
    Lazy::new(iced::widget::text_input::Id::unique);

fn handle_hotkey(key_code: Key, modifiers: Modifiers) -> Option<Message> {
    let key_code = key_code.as_ref();

    // Whatever the context, ctrl-Q quits the app
    if modifiers.control() && key_code == Key::Character("q") {
        return Some(Message::Quit);
    }

    // Keyboard shortcuts
    if modifiers.is_empty() {
        match key_code {
            Key::Character("l") => return Some(Message::ToggleLog),
            Key::Character("d") => return Some(Message::ToggleDebug),
            _ => {}
        }

        let some_key_code = Some(key_code.clone());

        for action in Action::all() {
            let shortcut = iced_shortcut_for(&action);

            if shortcut == some_key_code {
                return shortcut.map(Message::MaybeShortcut);
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
                .map(|result| result.to_string())
                .map(text)
                .map(Element::from),
        )
        .width(Length::Fill),
    )
}

fn simple_button(
    label: &str,
    message: Message,
) -> iced::widget::Button<'_, Message> {
    use iced::widget::{button, text};

    button(text(label).width(Length::Fill)).on_press(message)
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

fn iced_shortcut_for(action: &Action) -> Option<Key<&'static str>> {
    match action {
        Action::Accept => Some(Key::<&str>::Character("y")),
        Action::Always => Some(Key::<&str>::Character("a")),
        Action::Customize(_) => Some(Key::<&str>::Character("c")),
        Action::ViewAlternatives => None,
        Action::Replace(_) => None,
        Action::Skip => Some(Key::<&str>::Character("s")),
        Action::Refuse => Some(Key::<&str>::Character("r")),
        Action::Ignore => Some(Key::<&str>::Character("i")),
        Action::Abort => Some(Key::<&str>::Character("q")),
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
