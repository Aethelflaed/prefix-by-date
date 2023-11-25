#![cfg(feature = "cli")]

use crate::application::{Application, Confirmation};
use crate::cli::Cli;
use crate::matcher::Matcher;
use crate::replacement::Replacement;
use crate::ui::Interface;

use std::boxed::Box;
use std::path::Path;

use env_logger::Builder;
use indicatif::{MultiProgress, ProgressBar};

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Text {
    bar: Option<ProgressBar>,
    multi_progress: MultiProgress,
    matcher_name_length: usize,
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

    fn view(&self, app: &Application, path: &Path, _replacement: &Replacement) {
        use dialoguer::console::{pad_str, Alignment};

        for matcher in &app.matchers {
            if let Some(replacement) = matcher.check(path) {
                print!(
                    "{}: ",
                    pad_str(
                        matcher.name(),
                        self.matcher_name_length,
                        Alignment::Left,
                        None
                    )
                );

                self.present_replacement(&path, &replacement);
                println!("");
            }
        }
    }

    fn present_replacement(&self, path: &Path, replacement: &Replacement) {
        use dialoguer::console::style;
        use diff::Result::*;

        for diff in diff::chars(
            path.to_str().unwrap(),
            &replacement.new_path().unwrap().to_str().unwrap(),
        ) {
            match diff {
                Left(ch) => print!("{}", style(ch).red()),
                Right(ch) => print!("{}", style(ch).green()),
                Both(ch, _) => print!("{}", style(ch)),
            }
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

impl Interface for Text {
    fn setup_logger(&mut self, logger_builder: &mut Builder) -> LogResult {
        use indicatif_log_bridge::LogWrapper;

        let logger = logger_builder.build();

        LogWrapper::new(self.multi_progress.clone(), logger).try_init()
    }
    fn after_setup(&mut self, cli: &Cli, matchers: &Vec<Box<dyn Matcher>>) {
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
        path: &Path,
        replacement: &Replacement,
    ) -> Confirmation {
        use dialoguer::FuzzySelect;

        print!("Proceed with ");
        self.present_replacement(&path, &replacement);
        println!("?");

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
                self.view(app, path, replacement);
                self.confirm(app, path, replacement)
            }
            _ => todo!(),
        }
    }
}
