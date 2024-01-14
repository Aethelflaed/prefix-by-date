#![cfg(feature = "text")]

use crate::application::Result;
use crate::matcher::Matcher;
use crate::processing::{
    self, Communication, Confirmation, Error, Processing, Reporter,
};
use crate::replacement::Replacement;
use crate::ui::{
    self,
    actions::Action,
    state::{Current, State},
};

use std::boxed::Box;
use std::cell::RefCell;
use std::fmt;
use std::path::{Path, PathBuf};

use env_logger::Builder;

use dialoguer::theme::ColorfulTheme;
use indicatif::{MultiProgress, ProgressBar};

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Text {
    state: RefCell<State>,
    theme: ColorfulTheme,
    bar: Option<ProgressBar>,
    multi_progress: MultiProgress,
    matcher_name_length: usize,
    matchers: Vec<Box<dyn Matcher>>,
}

struct ReplacementDisplay<'a> {
    replacement: &'a Replacement,
}

impl<'a> fmt::Display for ReplacementDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use dialoguer::console::style;
        use diff::Result::*;

        for diff in diff::chars(
            self.replacement.file_stem.as_str(),
            self.replacement.new_file_stem.as_str(),
        ) {
            match diff {
                Left(ch) => write!(f, "{}", style(ch).red())?,
                Right(ch) => write!(f, "{}", style(ch).green())?,
                Both(ch, _) => write!(f, "{}", style(ch))?,
            }
        }

        Ok(())
    }
}

impl<'a> From<&'a Replacement> for ReplacementDisplay<'a> {
    fn from(replacement: &'a Replacement) -> ReplacementDisplay<'a> {
        Self { replacement }
    }
}

impl Text {
    /// Inidcate whether or not this interface is available
    pub fn available() -> bool {
        // If we are connected_to_journal, it means we're not connected to a
        // standard terminal so we can't really present
        !systemd_journal_logger::connected_to_journal() &&
            // If stdout is not a tty, then we probably don't want interaction
            // either
            atty::is(atty::Stream::Stdout)
    }

    #[cfg(test)]
    pub fn new() -> Self {
        // We need a hidden ProgressDrawTarget for the tests if we don't
        // want to polute the output
        Self::build(MultiProgress::with_draw_target(
            indicatif::ProgressDrawTarget::hidden(),
        ))
    }

    #[cfg(not(test))]
    pub fn new() -> Self {
        Self::build(MultiProgress::new())
    }

    fn build(multi_progress: MultiProgress) -> Self {
        Self {
            state: RefCell::<State>::default(),
            theme: ColorfulTheme::default(),
            multi_progress,
            bar: None,
            matcher_name_length: 0,
            matchers: Default::default(),
        }
    }

    fn inc_progress(&self) {
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        if let Some(bar) = &self.bar {
            bar.finish();
            self.multi_progress.remove(bar);
        }
    }
}

impl ui::Interface for Text {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        use indicatif_log_bridge::LogWrapper;

        let logger = logger_builder.build();

        LogWrapper::new(self.multi_progress.clone(), logger).try_init()
    }

    fn process(
        &mut self,
        matchers: &[Box<dyn Matcher>],
        paths: &[PathBuf],
    ) -> Result<()> {
        self.matchers = matchers.to_owned();

        self.state = RefCell::new(State::new(paths.len()));
        self.bar = Some(
            self.multi_progress
                .add(ProgressBar::new(paths.len() as u64)),
        );

        if let Some(matcher) = self
            .matchers
            .iter()
            .max_by_key(|matcher| matcher.name().len())
        {
            self.matcher_name_length = matcher.name().len();
        }

        Processing::new(self, matchers, paths).run()?;
        Ok(())
    }
}

impl Reporter for Text {
    fn setup(&self, _count: usize) {}
    fn processing(&self, path: &Path) {
        self.state.borrow_mut().set_current_path(path.to_path_buf());
    }
    fn processing_ok(&self, replacement: &Replacement) {
        self.state
            .borrow_mut()
            .set_current_success(replacement.clone());
        self.inc_progress();
    }
    fn processing_err(&self, path: &Path, error: &Error) {
        self.state
            .borrow_mut()
            .set_current_failure(path.to_path_buf(), format!("{}", error));
        self.inc_progress();
    }
}

impl Communication for Text {
    fn confirm(&self, replacement: &Replacement) -> Confirmation {
        let mut state = self.state.borrow_mut();
        state.set_current_confirm(replacement.clone(), &self.matchers);
        Resolver {
            ui: self,
            state: &mut state,
            action: None,
        }
        .resolve()
    }
    fn rescue(&self, error: Error) -> processing::Result<Replacement> {
        match &error {
            Error::NoMatch(path) => {
                let replacement = match Replacement::try_from(path.as_path()) {
                    Ok(rep) => rep,
                    Err(_) => return Err(error),
                };

                let mut state = self.state.borrow_mut();
                state.set_current_rescue(replacement.clone());
                let resolution = Resolver {
                    ui: self,
                    state: &mut state,
                    action: None,
                }
                .resolve();
                match resolution {
                    Confirmation::Abort => Err(Error::Abort),
                    Confirmation::Replace(replacement) => Ok(replacement),
                    Confirmation::Skip | Confirmation::Refuse => Err(error),
                    other => {
                        log::warn!(
                            "Unexpected rescue confirmation: {:?}",
                            other
                        );
                        Err(error)
                    }
                }
            }
            _ => {
                log::warn!("Unexpected rescue: {:?}", error);
                Err(error)
            }
        }
    }
}

struct Resolver<'a> {
    ui: &'a Text,
    state: &'a mut State,
    action: Option<Action>,
}

impl<'a> Resolver<'a> {
    fn resolve(&mut self) -> Confirmation {
        loop {
            if let Some(action) = self.action.take() {
                self.execute(action);
            } else {
                match self.state.current() {
                    Current::Confirm(change) => {
                        let rep = &change.replacement;

                        println!("In {}", rep.parent.display());
                        println!(
                            "Replace {} with {}",
                            rep.file_name(),
                            rep.new_file_name()
                        );

                        self.main_dialog();
                    }
                    Current::Rescue(change) => {
                        let rep = &change.replacement;

                        println!("In {}", rep.parent.display());
                        println!("No match was found for {}", rep.file_name());
                        self.main_dialog();
                    }
                    Current::Resolving(_, conf) => return conf.clone(),
                    Current::Path(_) | Current::None | Current::Resolved => {
                        unreachable!()
                    }
                };
            }
        }
    }

    fn main_dialog(&mut self) {
        use dialoguer::FuzzySelect;

        let mut prompts = vec![];
        let mut actions = vec![];

        for action in self.state.actions().iter() {
            if let Some(prompt) = self.prompt_for(action) {
                prompts.push(prompt);
                actions.push(action);
            }
        }

        let selection = FuzzySelect::with_theme(&self.ui.theme)
            .with_prompt("What do you want to do?")
            .items(&prompts)
            .interact()
            .unwrap();

        self.action = actions
            .get(selection)
            .map(|action_ref| (*action_ref).clone());
    }

    fn execute(&mut self, action: Action) {
        match action {
            Action::Accept
            | Action::Always
            | Action::Skip
            | Action::Refuse
            | Action::Ignore
            | Action::Abort => {
                self.state.set_current_resolving(
                    action
                        .clone()
                        .try_into()
                        .expect("Action convert to confirmation"),
                );
            }
            Action::ViewAlternatives => {
                self.view_alternatives();
            }
            Action::Customize(file_stem) => {
                self.customize(file_stem);
            }
            Action::Cancel => {
                self.state.cancel_customize();
            }
            Action::Replace(replacement) => {
                self.state
                    .set_current_resolving(Confirmation::Replace(replacement));
            }
            Action::ConfirmCustomization => {
                self.confirm_customization();
            }
        }
    }

    fn view_alternatives(&mut self) {
        use dialoguer::console::{pad_str, Alignment};
        use dialoguer::FuzzySelect;

        if let Current::Confirm(change) = self.state.current() {
            let mut replacements = vec![];
            let mut options = vec![];

            for (name, rep) in &change.alternatives {
                options.push(format!(
                    "{}: {} => {}",
                    pad_str(
                        name.as_str(),
                        self.ui.matcher_name_length,
                        Alignment::Left,
                        None
                    ),
                    rep.file_stem,
                    rep.new_file_stem
                ));
                replacements.push(rep);
            }

            options.push(String::from("Cancel"));

            let selection = FuzzySelect::with_theme(&self.ui.theme)
                .with_prompt("What do you want to do?")
                .items(&options)
                .interact()
                .unwrap();

            if let Some(replacement) = replacements.get(selection) {
                self.state.customize(replacement.new_file_stem.clone());
                self.action = Some(Action::ConfirmCustomization);
            }
        }
    }

    fn customize(&mut self, file_stem: String) {
        use dialoguer::Input;

        self.state.customize(file_stem.clone());

        if let Some(replacement) = self.state.customized_replacement() {
            let new_file_stem: String = Input::with_theme(&self.ui.theme)
                .with_prompt("New file name?")
                .with_initial_text(replacement.new_file_stem)
                .interact_text()
                .unwrap();

            self.state.customize(new_file_stem);
            self.action = Some(Action::ConfirmCustomization);
        }
    }

    fn confirm_customization(&mut self) {
        use dialoguer::FuzzySelect;

        if let Some(replacement) = self.state.customized_replacement() {
            let options = ["Yes", "No", "Customize"];

            let selection = FuzzySelect::with_theme(&self.ui.theme)
                .with_prompt(format!(
                    "Proceed with {}?",
                    ReplacementDisplay::from(&replacement)
                ))
                .items(&options)
                .interact()
                .unwrap();

            self.action = match selection {
                0 => Some(Action::Replace(replacement)),
                1 => Some(Action::Cancel),
                2 => Some(Action::Customize(replacement.new_file_stem.clone())),
                _ => None,
            }
        }
    }

    fn prompt_for(&self, action: &Action) -> Option<&'static str> {
        match action {
            Action::Accept => Some("Yes, accept the rename and continue"),
            Action::Always => Some("Always accept similar rename and continue"),
            Action::Customize(_) => Some("Customize the rename"),
            Action::ViewAlternatives => Some("View other possibilities"),
            Action::Replace(_) => None,
            Action::Skip => Some("Skip renaming this file"),
            Action::Refuse => {
                if let Current::Confirm(_) = self.state.current() {
                    Some("Refuse the rename and continue")
                } else {
                    None
                }
            }
            Action::Ignore => Some("Ignore all similar rename and continue"),
            Action::Abort => Some("Quit now, refusing this rename"),
            Action::Cancel => None,
            Action::ConfirmCustomization => None,
        }
    }
}
