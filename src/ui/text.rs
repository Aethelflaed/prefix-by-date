#![cfg(feature = "cli")]

use crate::application::Confirmation;
use crate::cli::Cli;
use crate::replacement::Replacement;
use crate::ui::Interface;

use std::path::Path;

use env_logger::Builder;
use indicatif::{MultiProgress, ProgressBar};

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Text {
    bar: Option<ProgressBar>,
    multi_progress: MultiProgress,
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
        }
    }

    #[cfg(not(test))]
    pub fn new() -> Self {
        let multi_progress = MultiProgress::new();

        Self {
            multi_progress,
            bar: None,
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
    fn after_setup(&mut self, cli: &Cli) {
        self.bar = Some(
            self.multi_progress
                .add(ProgressBar::new(cli.paths.len() as u64)),
        );
    }
    fn after_process(&self, _path: &Path) {
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }

    fn confirm(&self, path: &Path, replacement: &Replacement) -> Confirmation {
        use dialoguer::FuzzySelect;

        println!(
            "{} will be renamed into {}",
            path.display(),
            replacement.new_path().unwrap().to_str().unwrap()
        );

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
            0 => return Confirmation::Accept,
            1 => return Confirmation::Always,
            2 => return Confirmation::Skip,
            3 => return Confirmation::Refuse,
            4 => return Confirmation::Ignore,
            5 => return Confirmation::Abort,
            _ => todo!(),
        }
    }
}
