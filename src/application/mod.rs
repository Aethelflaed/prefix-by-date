use crate::cli::Cli;
use crate::matcher::{Matcher, Pattern, PredeterminedDate};
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
            ui: build_interface(&cli),
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

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.ui.process(&self.matchers, &self.cli.paths)
    }

    pub fn add_matcher(&mut self, matcher: Box<dyn Matcher>) {
        if !self.matchers.iter().any(|m| m.name() == matcher.name()) {
            self.matchers.push(matcher);
        }
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

fn build_interface(cli: &Cli) -> Box<dyn ui::Interface> {
    use crate::cli::Interactive;
    use crate::ui::{Gui, NonInteractive, Text};
    use systemd_journal_logger::connected_to_journal;

    // XXX check if we still connect to journal if we start with GUI via systemd
    if connected_to_journal() {
        return Box::new(NonInteractive::new());
    }

    match cli.interactive {
        Interactive::Text if cfg!(feature = "cli") => Box::new(Text::new()),
        Interactive::Gui if cfg!(feature = "gui") => Box::new(Gui::new()),
        _ => Box::new(NonInteractive::new()),
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

        let mut app = Application {
            cli,
            ..Application::default()
        };
        with_config(|| app.setup().unwrap());

        assert_eq!(3, app.matchers.len());
        assert_eq!("Predetermined date", app.matchers[0].name());
        assert_eq!("whatsapp", app.matchers[1].name());
        assert_eq!("cic", app.matchers[2].name());
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

        assert_eq!(2, app.matchers.len());
        assert_eq!("whatsapp", app.matchers[0].name());
        assert_eq!("%Y-%m-%d", app.matchers[0].date_format());
        assert_eq!("cic", app.matchers[1].name());
        assert_eq!("%Y-%m-%d", app.matchers[1].date_format());

        app = Application {
            cli: Cli {
                time: true,
                ..cli()
            },
            ..Application::default()
        };
        with_config(|| app.setup().unwrap());

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
