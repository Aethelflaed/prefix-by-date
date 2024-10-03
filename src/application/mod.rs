use crate::matcher::{Matcher, Metadata, Pattern, PredeterminedDate};
use crate::ui;

use std::boxed::Box;

mod cli;
pub use cli::Interactive;

mod arguments;
use arguments::Arguments;

pub use arguments::DEFAULT_DATE_FORMAT;
// The next symbol is only used during tests, which naturally causes the
// compiler to complain, but I still want to keep it available the same way
// as the previous one for the sake of uniformity
#[cfg(test)]
pub use arguments::DEFAULT_DATE_TIME_FORMAT;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Application {
    pub matchers: Vec<Box<dyn Matcher>>,
    ui: Box<dyn ui::Interface>,
    arguments: Arguments,
}

impl Default for Application {
    fn default() -> Self {
        use crate::ui::NonInteractive;

        Self {
            matchers: Vec::<Box<dyn Matcher>>::default(),
            arguments: Arguments::default(),
            ui: Box::new(NonInteractive::new()),
        }
    }
}

impl Application {
    pub fn new() -> Self {
        Self {
            arguments: Arguments::parse(),
            ..Self::default()
        }
    }

    pub fn setup(&mut self) -> Result<()> {
        self.setup_with_ui(ui::from(self.arguments.interactive()))
    }

    fn setup_with_ui(&mut self, ui: Box<dyn ui::Interface>) -> Result<()> {
        self.ui = ui;

        self.setup_log()?;
        log::set_max_level(self.arguments.log_level_filter());

        while let Some(error) = self.arguments.init_errors.pop_front() {
            log::info!("Init error: {}", error);
        }

        log::debug!("Arguments: {:?}", self.arguments);

        let format = self.arguments.default_format().to_string();

        if self.arguments.today() {
            self.add_matcher(PredeterminedDate::new(format.as_str()));
        }

        if let Some(patterns) = self.arguments.patterns.take() {
            patterns.iter().for_each(|(name, value)| {
                if let toml::Value::Table(table) = value {
                    if let Some(pattern) =
                        Pattern::deserialize(name, table, format.as_str())
                    {
                        self.add_pattern_matcher(pattern);
                    }
                }
            });
        }

        if self.arguments.metadata().created() {
            self.add_matcher(Metadata::new_created(format.as_str()));
        }
        if self.arguments.metadata().modified() {
            self.add_matcher(Metadata::new_modified(format.as_str()));
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        log::debug!(
            "Matchers: {:?}",
            self.matchers.iter().map(|m| m.name()).collect::<Vec<_>>()
        );
        log::debug!("Paths: {:?}", self.arguments.paths());
        self.ui.process(&self.matchers, self.arguments.paths())
    }

    pub(crate) fn add_pattern_matcher(&mut self, pattern: Pattern) {
        if pattern.time() == self.arguments.time()
            && !RESERVED_MATCHER_NAMES.contains(&pattern.name())
        {
            self.add_matcher(pattern);
        }
    }

    pub(crate) fn add_matcher<M: Matcher + 'static>(&mut self, matcher: M) {
        if !self.matchers.iter().any(|m| m.name() == matcher.name()) {
            self.matchers.push(Box::new(matcher));
        }
    }

    fn setup_log(&mut self) -> LogResult {
        use env_logger::{Builder, Env};
        use systemd_journal_logger::{connected_to_journal, JournalLog};

        // If the output streams of this process are directly connected to the
        // systemd journal log directly to the journal to preserve structured
        // log entries (e.g. proper multiline messages, metadata fields, etc.)
        if connected_to_journal() {
            JournalLog::new()
                .unwrap()
                .with_extra_fields(vec![("VERSION", env!("CARGO_PKG_VERSION"))])
                .install()
        } else {
            let name = String::from(env!("CARGO_PKG_NAME"))
                .replace('-', "_")
                .to_uppercase();
            let env = Env::new()
                .filter(format!("{}_LOG", name))
                .write_style(format!("{}_LOG_STYLE", name));

            self.ui.setup_logger(
                Builder::new()
                    .filter_level(log::LevelFilter::Trace)
                    .parse_env(env),
            )
        }
    }
}

const RESERVED_MATCHER_NAMES: [&str; 3] = [
    crate::matcher::predetermined_date::TODAY,
    crate::matcher::metadata::CREATED,
    crate::matcher::metadata::MODIFIED,
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{assert_eq, test};

    #[test]
    fn add_matcher_with_same_name() {
        let mut app = Application::default();

        assert_eq!(0, app.matchers.len());
        app.add_matcher(PredeterminedDate::default());
        assert_eq!(1, app.matchers.len());

        app.add_matcher(PredeterminedDate::default());
        assert_eq!(1, app.matchers.len());
    }

    mod add_pattern_matcher {
        use super::*;
        use crate::test::{assert_eq, test};

        #[test]
        fn refuse_reserved_matcher_names() {
            let mut app = Application::default();

            // Test failure cases
            for name in RESERVED_MATCHER_NAMES {
                app.add_pattern_matcher(
                    Pattern::builder().regex(".").name(name).build().unwrap(),
                );
                assert_eq!(0, app.matchers.len());
            }

            // And yet, it works
            app.add_pattern_matcher(
                Pattern::builder().regex(".").name("foo").build().unwrap(),
            );
            assert_eq!(1, app.matchers.len());
        }

        #[test]
        fn refuse_different_time_values() {
            let mut app = Application::default();

            // Test failure cases
            app.arguments.time = false;
            app.add_pattern_matcher(
                Pattern::builder()
                    .regex(".")
                    .name("foo")
                    .time(true)
                    .build()
                    .unwrap(),
            );
            assert_eq!(0, app.matchers.len());

            app.arguments.time = true;
            app.add_pattern_matcher(
                Pattern::builder()
                    .regex(".")
                    .name("foo")
                    .time(false)
                    .build()
                    .unwrap(),
            );
            assert_eq!(0, app.matchers.len());

            // And yet, it works
            app.add_pattern_matcher(
                Pattern::builder()
                    .regex(".")
                    .name("foo")
                    .time(true)
                    .build()
                    .unwrap(),
            );
            assert_eq!(1, app.matchers.len());
        }
    }

    mod setup {
        use super::*;
        use crate::test::test;
        use mockall::mock;

        use std::path::PathBuf;

        mock! {
            Interface {}

            impl ui::Interface for Interface {
                fn setup_logger(
                    &mut self,
                    _logger_builder: &mut env_logger::Builder,
                ) -> LogResult;

                fn process(
                    &mut self,
                    _matchers: &[Box<dyn crate::matcher::Matcher>],
                    _paths: &[PathBuf],
                ) -> Result<()>;
            }
        }

        #[test]
        fn default_run_with_fake_ui() {
            let mut app = Application::default();
            let mut ui = MockInterface::new();

            ui.expect_setup_logger().times(1).returning(|_| Ok(()));
            ui.expect_process().times(1).returning(|_, _| Ok(()));

            app.setup_with_ui(Box::new(ui)).unwrap();

            // by default, no matcher is added
            assert!(app.matchers.is_empty());

            app.run().unwrap();
        }

        #[test]
        fn setup_today_matcher() {
            let mut app = Application::default();
            let mut ui = MockInterface::new();

            ui.expect_setup_logger().times(1).returning(|_| Ok(()));

            use crate::matcher::predetermined_date::TODAY;
            app.arguments.today = true;

            app.setup_with_ui(Box::new(ui)).unwrap();

            assert!(app.matchers.iter().any(|m| m.name() == TODAY));
        }

        #[test]
        fn setup_created_matcher() {
            let mut app = Application::default();
            let mut ui = MockInterface::new();

            ui.expect_setup_logger().times(1).returning(|_| Ok(()));

            use crate::matcher::metadata::CREATED;
            use cli::Metadata;
            app.arguments.metadata = Metadata::Created;

            app.setup_with_ui(Box::new(ui)).unwrap();

            assert!(app.matchers.iter().any(|m| m.name() == CREATED));
        }

        #[test]
        fn setup_modified_matcher() {
            let mut app = Application::default();
            let mut ui = MockInterface::new();

            ui.expect_setup_logger().times(1).returning(|_| Ok(()));

            use crate::matcher::metadata::MODIFIED;
            use cli::Metadata;
            app.arguments.metadata = Metadata::Modified;

            app.setup_with_ui(Box::new(ui)).unwrap();

            assert!(app.matchers.iter().any(|m| m.name() == MODIFIED));
        }

        #[test]
        fn setup_metadata_both_matcher() {
            let mut app = Application::default();
            let mut ui = MockInterface::new();

            ui.expect_setup_logger().times(1).returning(|_| Ok(()));

            use crate::matcher::metadata::{CREATED, MODIFIED};
            use cli::Metadata;
            app.arguments.metadata = Metadata::Both;

            app.setup_with_ui(Box::new(ui)).unwrap();

            assert!(app.matchers.iter().any(|m| m.name() == CREATED));
            assert!(app.matchers.iter().any(|m| m.name() == MODIFIED));
        }
    }
}
