use crate::matcher::{Matcher, Metadata, Pattern, PredeterminedDate};
use crate::ui;

use std::boxed::Box;

mod cli;
pub use cli::Interactive;

mod arguments;
use arguments::Arguments;
pub use arguments::{DEFAULT_DATE_FORMAT, DEFAULT_DATE_TIME_FORMAT};

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
        self.ui = ui::from(self.arguments.interactive());

        self.setup_log()?;
        log::set_max_level(self.arguments.log_level_filter());
        log::debug!("Arguments: {:?}", self.arguments);

        let format = self.arguments.default_format().to_string();

        if self.arguments.today() {
            self.add_matcher(PredeterminedDate::new(format.as_str()));
        }

        let patterns = self.arguments.patterns().clone();
        patterns.iter().for_each(|(name, value)| {
            if let toml::Value::Table(table) = value {
                if let Some(pattern) =
                    Pattern::deserialize(name, table, format.as_str())
                {
                    self.add_pattern_matcher(pattern);
                }
            }
        });

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

    fn add_pattern_matcher(&mut self, pattern: Pattern) {
        if pattern.time() == self.arguments.time()
            && !RESERVED_MATCHER_NAMES.contains(&pattern.name())
        {
            self.add_matcher(pattern);
        }
    }

    fn add_matcher<M: Matcher + 'static>(&mut self, matcher: M) {
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
    use pretty_assertions::assert_eq;

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
        use pretty_assertions::assert_eq;

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
                Pattern::builder().regex(".").name("foo").time(true).build().unwrap(),
            );
            assert_eq!(0, app.matchers.len());

            app.arguments.time = true;
            app.add_pattern_matcher(
                Pattern::builder().regex(".").name("foo").time(false).build().unwrap(),
            );
            assert_eq!(0, app.matchers.len());

            // And yet, it works
            app.add_pattern_matcher(
                Pattern::builder().regex(".").name("foo").time(true).build().unwrap(),
            );
            assert_eq!(1, app.matchers.len());
        }
    }
}
