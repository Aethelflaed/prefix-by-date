#![cfg(feature = "cli")]

use crate::application::{Application, Confirmation};
use crate::cli::Cli;
use crate::matcher::Matcher;
use crate::replacement::Replacement;
use crate::ui::Interface;

use std::boxed::Box;
use std::fmt;
use std::path::Path;

use env_logger::Builder;
use indicatif::{MultiProgress, ProgressBar};

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Text {
    bar: Option<ProgressBar>,
    multi_progress: MultiProgress,
    matcher_name_length: usize,
}

struct ReplacementDisplay<'a> {
    replacement: &'a Replacement,
}

impl<'a> fmt::Display for ReplacementDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use dialoguer::console::style;
        use diff::Result::*;

        for diff in diff::chars(
            self.replacement.path.to_str().unwrap(),
            self.replacement.new_path().unwrap().to_str().unwrap(),
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
    #[cfg(test)]
    pub fn new() -> Self {
        // We need a hidden ProgressDrawTarget for the tests if we don't
        // want to polute the output
        let multi_progress = MultiProgress::with_draw_target(
            indicatif::ProgressDrawTarget::hidden(),
        );

        Self {
            multi_progress,
            bar: None,
            matcher_name_length: 0,
        }
    }

    #[cfg(not(test))]
    pub fn new() -> Self {
        let multi_progress = MultiProgress::new();

        Self {
            multi_progress,
            bar: None,
            matcher_name_length: 0,
        }
    }

    fn view(&self, app: &Application, replacement: &Replacement) {
        use dialoguer::console::{pad_str, Alignment};

        for matcher in &app.matchers {
            if let Some(replacement) = matcher.check(&replacement.path) {
                println!(
                    "{}: {}",
                    pad_str(
                        matcher.name(),
                        self.matcher_name_length,
                        Alignment::Left,
                        None
                    ),
                    ReplacementDisplay::from(&replacement)
                );
            }
        }
    }

    fn customize(&self, rep: &Replacement) -> Confirmation {
        use dialoguer::Input;

        let mut replacement = rep.clone();

        let new_file_stem: String = Input::new()
            .with_prompt("New file name?")
            .with_initial_text(replacement.new_file_stem)
            .interact_text()
            .unwrap();

        replacement.new_file_stem = new_file_stem;

        Confirmation::Replace(replacement)
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

impl Interface for Text {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        use indicatif_log_bridge::LogWrapper;

        let logger = logger_builder.build();

        LogWrapper::new(self.multi_progress.clone(), logger).try_init()
    }
    fn after_setup(&mut self, cli: &Cli, matchers: &[Box<dyn Matcher>]) {
        self.bar = Some(
            self.multi_progress
                .add(ProgressBar::new(cli.paths.len() as u64)),
        );

        if let Some(matcher) =
            matchers.iter().max_by_key(|matcher| matcher.name().len())
        {
            self.matcher_name_length = matcher.name().len();
        }
    }
    fn after_process(&self, _path: &Path) {
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }

    fn confirm(
        &self,
        app: &Application,
        replacement: &Replacement,
    ) -> Confirmation {
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

        let selection = FuzzySelect::new()
            .with_prompt("What do you want to do?")
            .items(&items)
            .interact()
            .unwrap();

        println!("You chose: {}", items[selection]);

        match selection {
            0 => Confirmation::Accept,
            1 => Confirmation::Always,
            2 => Confirmation::Skip,
            3 => Confirmation::Refuse,
            4 => Confirmation::Ignore,
            5 => Confirmation::Abort,
            6 => {
                self.view(app, replacement);
                self.confirm(app, replacement)
            }
            7 => self.customize(replacement),
            wtf => panic!("Unkown option {}", wtf),
        }
    }
}
