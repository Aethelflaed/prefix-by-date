#![cfg(feature = "text")]

use crate::application::Result;
use crate::matcher::Matcher;
use crate::processing::{
    self, Communication, Confirmation, Error, Processing, Reporter,
};
use crate::replacement::Replacement;
use crate::ui;

use std::boxed::Box;
use std::fmt;
use std::path::{Path, PathBuf};

use env_logger::Builder;

use dialoguer::theme::ColorfulTheme;
use indicatif::{MultiProgress, ProgressBar};

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Text {
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

    fn confirm(&self, replacement: &Replacement) -> Confirmation {
        use dialoguer::FuzzySelect;

        println!("Proceed with {}?", ReplacementDisplay::from(replacement));

        let items = vec![
            "Yes, accept the rename and continue",
            "Always accept similar rename and continue",
            "Skip renaming this file",
            "Refuse the rename and continue",
            "Ignore all similar rename and continue",
            "Quit now, refusing this rename",
            "View other possibilities",
            "Customize the rename",
        ];

        let selection = FuzzySelect::with_theme(&self.theme)
            .with_prompt("What do you want to do?")
            .items(&items)
            .interact()
            .unwrap();

        match selection {
            0 => Confirmation::Accept,
            1 => Confirmation::Always,
            2 => Confirmation::Skip,
            3 => Confirmation::Refuse,
            4 => Confirmation::Ignore,
            5 => Confirmation::Abort,
            6 => match self.view(replacement) {
                Confirmation::Abort => self.confirm(replacement),
                other => other,
            },
            7 => match self.customize(replacement) {
                Confirmation::Abort => self.confirm(replacement),
                other => other,
            },
            wtf => panic!("Unkown option {}", wtf),
        }
    }

    fn view(&self, replacement: &Replacement) -> Confirmation {
        use dialoguer::console::{pad_str, Alignment};
        use dialoguer::FuzzySelect;

        let mut replacements: Vec<Replacement> = vec![];
        let mut options: Vec<String> = vec![];

        for matcher in &self.matchers {
            if let Some(replacement) =
                matcher.check(replacement.path().as_path())
            {
                options.push(format!(
                    "{}: {}",
                    pad_str(
                        matcher.name(),
                        self.matcher_name_length,
                        Alignment::Left,
                        None
                    ),
                    replacement
                ));
                replacements.push(replacement);
            }
        }

        options.push(String::from("Abort"));

        let selection = FuzzySelect::with_theme(&self.theme)
            .with_prompt("What do you want to do?")
            .items(&options)
            .interact()
            .unwrap();

        if let Some(replacement) = replacements.get(selection) {
            Confirmation::Replace(replacement.clone())
        } else {
            Confirmation::Abort
        }
    }

    fn customize(&self, rep: &Replacement) -> Confirmation {
        use dialoguer::{Confirm, Input};

        let mut replacement = rep.clone();

        let new_file_stem: String = Input::with_theme(&self.theme)
            .with_prompt("New file name?")
            .with_initial_text(replacement.new_file_stem)
            .interact_text()
            .unwrap();

        replacement.new_file_stem = new_file_stem;

        let confirmed = Confirm::with_theme(&self.theme)
            .with_prompt(format!(
                "Proceed with {}?",
                ReplacementDisplay::from(&replacement)
            ))
            .interact()
            .unwrap();

        if confirmed {
            Confirmation::Replace(replacement)
        } else {
            Confirmation::Abort
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
    fn processing(&self, _path: &Path) {}
    fn processing_ok(&self, _replacement: &Replacement) {
        self.inc_progress();
    }
    fn processing_err(&self, _path: &Path, _error: &Error) {
        self.inc_progress();
    }
}

impl Communication for Text {
    fn confirm(&self, replacement: &Replacement) -> Confirmation {
        Text::confirm(self, replacement)
    }
    fn rescue(&self, error: Error) -> processing::Result<Replacement> {
        match &error {
            Error::NoMatch(path) => {
                let replacement = match Replacement::try_from(path.as_path()) {
                    Ok(rep) => rep,
                    Err(_) => return Err(error),
                };
                println!("No match found for {:?}.", path);
                match self.customize(&replacement) {
                    Confirmation::Abort => Err(error),
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
