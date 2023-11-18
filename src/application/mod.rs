mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

use crate::cli::Cli;
use crate::matcher::{Matcher, Pattern, PredeterminedDate};
use crate::processing;
use crate::replacement::Replacement;
use crate::reporter::{Log, Reporter};
use crate::ui::Interface;

use std::boxed::Box;
use std::path::{Path, PathBuf};

use toml::Table;

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Application {
    pub matchers: Vec<Box<dyn Matcher>>,
    pub reporters: Vec<Box<dyn Reporter>>,
    pub cli: Cli,
    interface: Box<dyn Interface>,
}

#[allow(dead_code)]
pub enum Confirmation {
    Accept,
    Always,
    Skip,
    Refuse,
    Ignore,
    Abort,
    Replace(Replacement),
}

impl Default for Application {
    fn default() -> Self {
        use crate::ui::NonInteractive;

        Self {
            matchers: Vec::<Box<dyn Matcher>>::default(),
            reporters: Vec::<Box<dyn Reporter>>::default(),
            cli: Cli::default(),
            interface: Box::new(NonInteractive::new()),
        }
    }
}

impl Application {
    pub fn new() -> Result<Self> {
        use clap::Parser;

        let cli = Cli::parse();

        let mut app = Self {
            interface: build_interface(&cli),
            cli,
            ..Self::default()
        };

        app.setup_log()?;

        Ok(app)
    }

    pub fn setup(&mut self) -> Result<()> {
        log::set_max_level(self.cli.verbose.log_level_filter());

        let mut format = "%Y-%m-%d";
        if self.cli.time {
            log::debug!("Prefix by date and time");
            format = "%Y-%m-%d %Hh%Mm%S";
        }

        if self.cli.today {
            log::debug!("Prefix by today's date");

            self.matchers.push(Box::new(PredeterminedDate::new(format)));
        }

        self.read_config(format)?;
        self.add_reporter(Box::<Log>::default());
        self.interface.after_setup(&self.cli);

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        use crate::processing::Processing;

        Processing::new(self).run(&self.cli.paths)?;

        Ok(())
    }

    pub fn add_matcher(&mut self, matcher: Box<dyn Matcher>) {
        if !self.matchers.iter().any(|m| m.name() == matcher.name()) {
            self.matchers.push(matcher);
        }
    }

    pub fn add_reporter(&mut self, reporter: Box<dyn Reporter>) {
        self.reporters.push(reporter);
    }

    pub fn confirm(
        &self,
        path: &Path,
        replacement: &Replacement,
    ) -> Confirmation {
        self.interface.confirm(path, replacement)
    }

    fn read_config(&mut self, default_format: &str) -> std::io::Result<()> {
        let file = config_home().join("patterns.toml");

        std::fs::read_to_string(file).map(|content| {
            content.parse::<Table>().unwrap().iter().for_each(
                |(name, value)| {
                    if let toml::Value::Table(table) = value {
                        if let Some(pattern) =
                            Pattern::deserialize(name, table, default_format)
                        {
                            self.add_matcher(Box::new(pattern));
                        }
                    }
                },
            );
        })
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
            let env = Env::new()
                .filter(format!("{}_LOG", env!("CARGO_PKG_NAME")))
                .write_style(format!("{}_LOG_STYLE", env!("CARGO_PKG_NAME")));

            self.interface.setup_logger(
                Builder::new()
                    .filter_level(log::LevelFilter::Trace)
                    .parse_env(env),
            )
        }
    }
}

impl Reporter for Application {
    fn count(&self, number: usize) {
        for reporter in &self.reporters {
            reporter.count(number);
        }
    }

    fn processing(&self, path: &Path) {
        for reporter in &self.reporters {
            reporter.processing(path);
        }
    }

    fn processing_err(&self, path: &Path, error: &processing::Error) {
        self.interface.after_process(path);

        for reporter in &self.reporters {
            reporter.processing_err(path, error);
        }
    }

    fn processing_ok(&self, path: &Path, new_path: &Path) {
        self.interface.after_process(path);

        for reporter in &self.reporters {
            reporter.processing_ok(path, new_path);
        }
    }
}

fn config_home() -> PathBuf {
    match std::env::var("PREFIX_BY_DATE_CONFIG") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
            .unwrap()
            .get_config_home(),
    }
}

#[cfg(not(feature = "cli"))]
fn build_interface(_cli: &Cli) -> Box<dyn Interface> {
    use crate::ui::NonInteractive;

    Box::new(NonInteractive::new())
}

#[cfg(feature = "cli")]
fn build_interface(cli: &Cli) -> Box<dyn Interface> {
    use crate::cli::Interactive;
    use crate::ui::{text::Text, NonInteractive};
    use systemd_journal_logger::connected_to_journal;

    if connected_to_journal() {
        return Box::new(NonInteractive::new());
    }
    if let Interactive::Off = cli.interactive {
        return Box::new(NonInteractive::new());
    }

    Box::new(Text::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{fixture::FileWriteStr, fixture::PathChild, TempDir};
    use pretty_assertions::assert_eq;
    use temp_env::with_var;

    fn cli() -> Cli {
        Cli {
            verbose: clap_verbosity_flag::Verbosity::new(0, 0),
            today: false,
            time: false,
            paths: vec![],
        }
    }

    fn with_config<T, R>(function: T) -> R
    where
        T: FnOnce() -> R,
    {
        let temp = TempDir::new().unwrap();
        let result = with_var(
            "PREFIX_BY_DATE_CONFIG",
            Some(temp.path().as_os_str()),
            || {
                temp.child("patterns.toml")
                    .write_str(
                        r#"
[whatsapp]
regex = """
  [A-Z]+-
  (?<year>\\d{4})
  (?<month>\\d{2})
  (?<day>\\d{2})
  -
  (?<rest>.+)
"""

[cic]
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
    fn today() {
        let cli = Cli {
            today: true,
            ..cli()
        };

        let mut app = Application::default();
        with_config(|| app.setup(&cli).unwrap());

        assert_eq!(3, app.matchers.len());
        assert_eq!("Predetermined date", app.matchers[0].name());
        assert_eq!("whatsapp", app.matchers[1].name());
        assert_eq!("cic", app.matchers[2].name());
    }

    #[test]
    fn time() {
        with_config(|| {
            let mut cli = cli();
            let mut app = Application::default();
            app.setup(&cli).unwrap();
            assert_eq!("%Y-%m-%d", app.matchers[0].date_format());

            cli.time = true;
            app = Application::default();
            app.setup(&cli).unwrap();
            assert_eq!("%Y-%m-%d %Hh%Mm%S", app.matchers[0].date_format());
        })
    }

    #[test]
    fn config_home_value() {
        with_var("PREFIX_BY_DATE_CONFIG", None::<&str>, || {
            let xdg_dirs =
                xdg::BaseDirectories::with_prefix("prefix-by-date").unwrap();
            assert_eq!(xdg_dirs.get_config_home(), config_home());
        });

        with_var("PREFIX_BY_DATE_CONFIG", Some("./"), || {
            assert_eq!(PathBuf::from("./"), config_home());
        });
    }

    #[test]
    fn read_config() {
        let mut cli = cli();
        let mut app = Application::default();
        with_config(|| app.setup(&cli).unwrap());

        assert_eq!(2, app.matchers.len());
        assert_eq!("whatsapp", app.matchers[0].name());
        assert_eq!("%Y-%m-%d", app.matchers[0].date_format());
        assert_eq!("cic", app.matchers[1].name());
        assert_eq!("%Y-%m-%d", app.matchers[1].date_format());

        cli.time = true;
        app = Application::default();
        with_config(|| app.setup(&cli).unwrap());

        assert_eq!(2, app.matchers.len());
        assert_eq!("whatsapp", app.matchers[0].name());
        assert_eq!("%Y-%m-%d %Hh%Mm%S", app.matchers[0].date_format());
        assert_eq!("cic", app.matchers[1].name());
        assert_eq!("%Y-%m-%d %Hh%Mm%S", app.matchers[1].date_format());
    }

    #[test]
    fn add_matcher_with_same_name() {
        let mut app = Application::default();

        app.add_matcher(Box::<PredeterminedDate>::default());
        assert_eq!(1, app.matchers.len());

        app.add_matcher(Box::<PredeterminedDate>::default());
        assert_eq!(1, app.matchers.len());
    }
}
