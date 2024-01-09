use crate::cli::Cli;
use crate::matcher::{Matcher, Metadata, Pattern, PredeterminedDate};
use crate::ui;

use std::boxed::Box;
use std::path::PathBuf;

use toml::Table;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

type LogResult = std::result::Result<(), log::SetLoggerError>;

pub struct Application {
    pub matchers: Vec<Box<dyn Matcher>>,
    pub cli: Cli,
    ui: Box<dyn ui::Interface>,
}

impl Default for Application {
    fn default() -> Self {
        use crate::ui::NonInteractive;

        Self {
            matchers: Vec::<Box<dyn Matcher>>::default(),
            cli: Cli::default(),
            ui: Box::new(NonInteractive::new()),
        }
    }
}

impl Application {
    pub fn new() -> Result<Self> {
        use clap::Parser;

        let cli = Cli::parse();

        let mut app = Self {
            ui: ui::from(cli.interactive),
            cli,
            ..Self::default()
        };

        app.setup_log()?;

        Ok(app)
    }

    pub fn setup(&mut self) -> Result<()> {
        log::set_max_level(self.cli.verbose.log_level_filter());
        let matchers = self.read_config()?;

        let mut format = "%Y-%m-%d";
        if self.cli.time {
            log::debug!("Prefix by date and time");
            format = "%Y-%m-%d %Hh%Mm%S";
        }

        if self.cli.today {
            log::debug!("Prefix by today's date");

            self.matchers
                .push(Box::new(PredeterminedDate::new(format, self.cli.time)));
        }

        self.matchers
            .push(Box::new(Metadata::new_created(format, self.cli.time)));
        self.matchers
            .push(Box::new(Metadata::new_modified(format, self.cli.time)));


        matchers.iter().for_each(|(name, value)| {
            if let toml::Value::Table(table) = value {
                if let Some(pattern) =
                    Pattern::deserialize(name, table, format)
                {
                    self.add_matcher(Box::new(pattern));
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
        log::debug!("Paths: {:?}", self.cli.paths);
        self.ui.process(&self.matchers, &self.cli.paths)
    }

    pub fn add_matcher(&mut self, matcher: Box<dyn Matcher>) {
        if matcher.time() == self.cli.time
            && !self.matchers.iter().any(|m| m.name() == matcher.name())
        {
            self.matchers.push(matcher);
        }
    }

    fn read_config(&mut self) -> std::io::Result<Table> {
        use toml::Value;

        let file = config_home().join("config.toml");

        let mut matchers: Table = Default::default();

        std::fs::read_to_string(file).map(|content| {
            let table = content.parse::<Table>().expect("Parse config as toml");

            if let Some(table) = table.get("matchers").and_then(Value::as_table).cloned() {
                matchers = table;
            }
        })?;

        Ok(matchers)
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

fn config_home() -> PathBuf {
    match std::env::var("PREFIX_BY_DATE_CONFIG") {
        Ok(val) if !val.is_empty() => PathBuf::from(val),
        _ => xdg::BaseDirectories::with_prefix(env!("CARGO_PKG_NAME"))
            .unwrap()
            .get_config_home(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::{fixture::FileWriteStr, fixture::PathChild, TempDir};
    use pretty_assertions::assert_eq;
    use temp_env::with_var;

    fn cli() -> Cli {
        use crate::cli::Interactive;

        Cli {
            verbose: clap_verbosity_flag::Verbosity::new(0, 0),
            today: false,
            time: false,
            interactive: Interactive::Off,
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
    fn today() {
        let cli = Cli {
            today: true,
            ..cli()
        };

        let mut app = Application {
            cli,
            ..Application::default()
        };
        with_config(|| app.setup().unwrap());

        assert_eq!(5, app.matchers.len());
        assert_eq!("Predetermined date", app.matchers[0].name());
        assert_eq!("created", app.matchers[1].name());
        assert_eq!("modified", app.matchers[2].name());
        assert_eq!("whatsapp", app.matchers[3].name());
        assert_eq!("cic", app.matchers[4].name());
    }

    #[test]
    fn time() {
        with_config(|| {
            let mut app = Application {
                cli: cli(),
                ..Application::default()
            };
            app.setup().unwrap();
            assert_eq!("%Y-%m-%d", app.matchers[0].date_format());

            app = Application {
                cli: Cli {
                    time: true,
                    ..cli()
                },
                ..Application::default()
            };
            app.setup().unwrap();
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
        let mut app = Application {
            cli: cli(),
            ..Application::default()
        };
        with_config(|| app.setup().unwrap());

        assert_eq!(4, app.matchers.len());
        assert_eq!("whatsapp", app.matchers[2].name());
        assert_eq!("%Y-%m-%d", app.matchers[2].date_format());
        assert_eq!("cic", app.matchers[3].name());
        assert_eq!("%Y-%m-%d", app.matchers[3].date_format());

        app = Application {
            cli: Cli {
                time: true,
                ..cli()
            },
            ..Application::default()
        };
        with_config(|| app.setup().unwrap());

        assert_eq!(3, app.matchers.len());
        assert_eq!("created", app.matchers[0].name());
        assert_eq!("modified", app.matchers[1].name());
        assert_eq!("whatsapp_time", app.matchers[2].name());
        assert_eq!("%Y-%m-%d %Hh%Mm%S", app.matchers[2].date_format());
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
