use crate::matcher::{Matcher, Metadata, Pattern, PredeterminedDate};
use crate::ui;

use std::boxed::Box;

mod cli;
pub use cli::Interactive;

mod arguments;
use arguments::Arguments;

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
            self.matchers
                .push(Box::new(PredeterminedDate::new(format.as_str())));
        }

        if self.arguments.metadata().created() {
            self.matchers
                .push(Box::new(Metadata::new_created(format.as_str())));
        }
        if self.arguments.metadata().modified() {
            self.matchers
                .push(Box::new(Metadata::new_modified(format.as_str())));
        }

        let patterns = self.arguments.patterns().clone();
        patterns.iter().for_each(|(name, value)| {
            if let toml::Value::Table(table) = value {
                if let Some(pattern) =
                    Pattern::deserialize(name, table, format.as_str())
                {
                    if pattern.time() == self.arguments.time() {
                        self.add_matcher(Box::new(pattern));
                    }
                }
            }
        });

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

    pub fn add_matcher(&mut self, matcher: Box<dyn Matcher>) {
        if !self.matchers.iter().any(|m| m.name() == matcher.name()) {
            self.matchers.push(matcher);
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

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{fixture::FileWriteStr, fixture::PathChild, TempDir};
    use pretty_assertions::assert_eq;
    use temp_env::with_var;

    fn with_config<T, R>(function: T) -> R
    where
        T: FnOnce() -> R,
    {
        let temp = TempDir::new().unwrap();
        let result = with_var(
            "PREFIX_BY_DATE_CONFIG",
            Some(temp.path().as_os_str()),
            || {
                temp.child("config.toml")
                    .write_str(
                        r#"
[matchers.whatsapp_time]
regex = """
  [A-Z]+-
  (?<year>\\d{4})
  (?<month>\\d{2})
  (?<day>\\d{2})
  (?<hour>\\d{2})
  (?<min>\\d{2})
  (?<sec>\\d{2})
  -
  (?<rest>.+)
"""
time = true

[matchers.whatsapp]
regex = """
  [A-Z]+-
  (?<year>\\d{4})
  (?<month>\\d{2})
  (?<day>\\d{2})
  -
  (?<rest>.+)
"""

[matchers.cic]
regex = """
  (?<rest>.+)
  \\s+au\\s+
  (?<year>\\d{4})
  -
  (?<month>\\d{2})
  -
  (?<day>\\d{2})
"""
"#,
                    )
                    .unwrap();

                function()
            },
        );
        temp.close().unwrap();

        return result;
    }

    #[test]
    fn add_matcher_with_same_name() {
        let mut app = Application::default();

        assert_eq!(0, app.matchers.len());
        app.add_matcher(Box::<PredeterminedDate>::default());
        assert_eq!(1, app.matchers.len());

        app.add_matcher(Box::<PredeterminedDate>::default());
        assert_eq!(1, app.matchers.len());
    }
}
