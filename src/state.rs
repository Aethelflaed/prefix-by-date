use crate::cli::Cli;
use crate::matcher::{Matcher, Pattern, PredeterminedDate};
use chrono::Local;
use std::boxed::Box;
use std::path::PathBuf;
use toml::Table;

#[derive(Default)]
pub struct State {
    pub format: String,
    pub matchers: Vec<Box<dyn Matcher>>,
}

impl State {
    pub fn from(cli: &Cli) -> std::io::Result<Self> {
        let mut state = State {
            format: "%Y-%m-%d".into(),
            ..State::default()
        };

        if cli.time {
            log::debug!("Prefix by date and time");
            state.format = "%Y-%m-%d %Hh%Mm%S".into();
        }

        if cli.today {
            log::debug!("Prefix by today's date");

            state.matchers.push(Box::new(PredeterminedDate {
                date_time: Local::now(),
            }));
        }

        state.read_config()?;

        for matcher in &state.matchers {
            log::debug!("Using matcher: {}", matcher.name());
        }

        Ok(state)
    }

    fn read_config(&mut self) -> std::io::Result<()> {
        let file = config_home().join("patterns.toml");

        std::fs::read_to_string(file).map(|content| {
            content
                .parse::<Table>()
                .unwrap()
                .iter()
                .for_each(|(name, value)| {
                    if let toml::Value::Table(table) = value {
                        if let Some(pattern) = Pattern::deserialize(name, table) {
                            self.add_matcher(Box::new(pattern));
                        }
                    }
                });
        })
    }

    fn add_matcher(&mut self, matcher: Box<dyn Matcher>) {
        if !self.matchers.iter().any(|m| m.name() == matcher.name()) {
            self.matchers.push(matcher);
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
        Cli {
            verbose: clap_verbosity_flag::Verbosity::new(0, 0),
            today: false,
            time: false,
            files: vec![],
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

        let state = with_config(|| State::from(&cli).unwrap());

        assert_eq!(3, state.matchers.len());
        assert_eq!("Predetermined date", state.matchers[0].name());
        assert_eq!("whatsapp", state.matchers[1].name());
        assert_eq!("cic", state.matchers[2].name());
    }

    #[test]
    fn time() {
        with_config(|| {
            let mut cli = cli();
            let mut state = State::from(&cli).unwrap();
            assert_eq!("%Y-%m-%d", state.format);

            cli.time = true;
            state = State::from(&cli).unwrap();
            assert_eq!("%Y-%m-%d %Hh%Mm%S", state.format);
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
        let state = with_config(|| State::from(&cli()).unwrap());

        assert_eq!(2, state.matchers.len());
        assert_eq!("whatsapp", state.matchers[0].name());
        assert_eq!("cic", state.matchers[1].name());
    }

    #[test]
    fn add_matcher_with_same_name() {
        let mut state = State::default();

        state.add_matcher(Box::new(PredeterminedDate { date_time: Local::now() }));
        assert_eq!(1, state.matchers.len());

        state.add_matcher(Box::new(PredeterminedDate { date_time: Local::now() }));
        assert_eq!(1, state.matchers.len());
    }
}
