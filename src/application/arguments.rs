use crate::application::cli::{Cli, Interactive, Metadata};
use crate::application::Error;

use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;

use toml::{Table, Value};

#[derive(Debug)]
pub struct Arguments {
    /// Command-line interface arguments
    cli: Cli,

    pub(in crate::application) time: bool,

    default_date_format: String,
    default_date_time_format: String,

    today: bool,
    metadata: Metadata,

    pub(in crate::application) patterns: Option<Table>,

    pub(in crate::application) init_errors: VecDeque<Error>,
}

impl Default for Arguments {
    fn default() -> Self {
        Self {
            cli: Cli::default(),
            time: false,
            default_date_format: String::from(DEFAULT_DATE_FORMAT),
            default_date_time_format: String::from(DEFAULT_DATE_TIME_FORMAT),
            today: false,
            metadata: Metadata::default(),
            patterns: None,
            init_errors: VecDeque::<Error>::default(),
        }
    }
}

pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d";
pub const DEFAULT_DATE_TIME_FORMAT: &str = "%Y-%m-%d %Hh%Mm%S";

impl Arguments {
    pub fn parse() -> Self {
        match Self::try_parse_from(std::env::args_os()) {
            Ok(args) => args,
            Err(error) => error.exit(),
        }
    }

    fn try_parse_from<I, T>(iter: I) -> std::result::Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        use clap::Parser;

        let mut instance = Arguments::default();
        instance.cli.try_update_from(iter)?;
        instance.apply_config("config.toml");
        instance.apply_cli();

        Ok(instance)
    }

    pub fn log_level_filter(&self) -> log::LevelFilter {
        self.cli.verbose.log_level_filter()
    }

    pub fn interactive(&self) -> Interactive {
        self.cli.interactive
    }

    /// Prefix by date and time if true, date only otherwise
    pub fn time(&self) -> bool {
        self.time
    }

    /// Default format string to format date
    pub fn default_format(&self) -> &str {
        match self.time() {
            true => &self.default_date_time_format,
            false => &self.default_date_format,
        }
    }

    /// Use pre-determined date matcher with today's date
    pub fn today(&self) -> bool {
        self.today
    }

    /// Use metadata matchers (creation and modification time)
    pub fn metadata(&self) -> Metadata {
        self.metadata
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.cli.paths
    }

    fn apply_cli(&mut self) {
        if let Some(time) = self.cli.time() {
            self.time = time;
        }

        if let Some(metadata) = self.cli.metadata {
            self.metadata = metadata;
        }

        self.today = self.cli.today;
    }

    fn apply_config(&mut self, filename: &str) {
        let dir = self.cli.config.take().unwrap_or_else(config_home);
        let path = dir.join(filename);

        match std::fs::read_to_string(path) {
            Ok(content) => match content.parse::<Table>() {
                Ok(config_table) => self.apply_config_table(config_table),
                Err(e) => self.init_errors.push_back(
                    format!("Unable to parse config file: {:?}", e).into(),
                ),
            },
            Err(e) => self.init_errors.push_back(
                format!("Unable to read config file: {:?}", e).into(),
            ),
        }
    }

    fn apply_config_table(&mut self, mut config_table: Table) {
        if let Some(value) = config_table.get("time").and_then(Value::as_bool) {
            self.time = value;
        }

        if let Some(Value::Table(mut formats)) =
            config_table.remove("default_format")
        {
            if let Some(Value::String(format)) = formats.remove("date") {
                self.default_date_format = format;
            }
            if let Some(Value::String(format)) = formats.remove("date_time") {
                self.default_date_time_format = format;
            }
        }

        if let Some(Value::Table(mut matchers)) =
            config_table.remove("matchers")
        {
            if let Some(Value::Table(predet)) =
                matchers.remove("predetermined_date")
            {
                if let Some(today) =
                    predet.get("today").and_then(Value::as_bool)
                {
                    self.today = today;
                }
            }

            if let Some(Value::Table(metadata)) = matchers.remove("metadata") {
                let created = metadata.get("created").and_then(Value::as_bool);
                let modified =
                    metadata.get("modified").and_then(Value::as_bool);

                if matches!(self.metadata, Metadata::None) {
                    match (created, modified) {
                        (Some(true), Some(true)) => {
                            self.metadata = Metadata::Both
                        }
                        (Some(true), _) => self.metadata = Metadata::Created,
                        (_, Some(true)) => self.metadata = Metadata::Modified,
                        (_, _) => {}
                    };
                } else {
                    self.init_errors.push_back(
                        format!(
                            "Unexpected metadata state on parse_config: {:?}",
                            self.metadata
                        )
                        .into(),
                    );
                }
            }

            if let Some(Value::Table(patterns)) = matchers.remove("patterns") {
                self.patterns = Some(patterns);
            }
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
    use assert_fs::{fixture::PathCopy, TempDir};
    use pretty_assertions::assert_eq;
    use temp_env::with_var;

    fn fixtures_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn with_config_dir<T, R>(function: T) -> R
    where
        T: FnOnce(&TempDir) -> R,
    {
        let temp = TempDir::new().unwrap();
        let result = with_var(
            "PREFIX_BY_DATE_CONFIG",
            Some(temp.path().as_os_str()),
            || function(&temp),
        );

        // The descrutor would silence any issue, so we call close() explicitly
        temp.close().unwrap();

        result
    }

    fn with_config_copied<T, R, S>(patterns: &[S], function: T) -> R
    where
        T: FnOnce() -> R,
        S: AsRef<str>,
    {
        with_config_dir(|temp| {
            temp.copy_from(fixtures_path(), patterns).unwrap();

            function()
        })
    }

    fn with_config<T, R>(function: T) -> R
    where
        T: FnOnce() -> R,
    {
        with_config_copied(&["config.toml"], function)
    }

    fn arguments_with_config(config: &str) -> Arguments {
        let mut arguments = Arguments::default();

        with_config_copied(&[config], || {
            arguments.apply_config(config);
        });

        arguments
    }

    #[test]
    fn config_home_default() {
        with_var("PREFIX_BY_DATE_CONFIG", None::<&str>, || {
            let xdg_dirs =
                xdg::BaseDirectories::with_prefix("prefix-by-date").unwrap();
            assert_eq!(xdg_dirs.get_config_home(), config_home());
        });
    }

    #[test]
    fn config_home_with_var() {
        with_var("PREFIX_BY_DATE_CONFIG", Some("./"), || {
            assert_eq!(PathBuf::from("./"), config_home());
        });
    }

    #[test]
    fn default_format() {
        let mut arguments = Arguments::default();

        assert_eq!(DEFAULT_DATE_FORMAT, arguments.default_format());

        arguments.time = true;
        assert_eq!(DEFAULT_DATE_TIME_FORMAT, arguments.default_format());
    }

    #[test]
    fn invalid_cli_args() {
        assert!(matches!(
            Arguments::try_parse_from(&["arg0", "--foo"]),
            Err(_)
        ));
    }

    #[test]
    fn try_parse_from() {
        let arguments = with_config(|| Arguments::parse());
        assert!(!arguments.time());

        let arguments = with_config(|| {
            Arguments::try_parse_from(&["arg0", "--time"]).unwrap()
        });
        assert!(arguments.time());

        let arguments = with_config(|| {
            Arguments::try_parse_from(&["arg0", "--no-time"]).unwrap()
        });
        assert!(!arguments.time());

        let arguments = with_config(|| {
            Arguments::try_parse_from(&["arg0", "--today"]).unwrap()
        });
        assert!(arguments.today());

        let arguments = with_config(|| {
            Arguments::try_parse_from(&["arg0", "--metadata", "both"]).unwrap()
        });
        assert!(matches!(arguments.metadata(), Metadata::Both));

        let arguments = with_config(|| {
            Arguments::try_parse_from(&["arg0", "--metadata=created"]).unwrap()
        });
        assert!(matches!(arguments.metadata(), Metadata::Created));
    }

    #[test]
    fn parse_with_cli_config() {
        let mut arguments = with_config_dir(|dir| {
            Arguments::try_parse_from(&[
                "arg0",
                "-C",
                dir.path().to_str().unwrap(),
            ])
            .unwrap()
        });

        match arguments.init_errors.pop_front() {
            Some(Error::Custom(string)) => {
                assert!(
                    string.starts_with("Unable to read config file"),
                    "String predicate failed for: {string:?}"
                );
            }
            Some(error) => assert!(false, "Unknown error: {error:?}"),
            None => {
                assert!(false, "An error was expected but none was received")
            }
        };
    }

    #[test]
    fn paths() {
        let arguments = with_config(|| {
            Arguments::try_parse_from(&["arg0", "foo", "bar"]).unwrap()
        });

        assert_eq!(
            [PathBuf::from("foo"), PathBuf::from("bar"),],
            arguments.paths()
        );
    }

    mod apply_config {
        use super::*;
        use pretty_assertions::assert_eq;

        #[test]
        fn fails_silently_on_missing_config() {
            let mut arguments = Arguments::default();

            with_config_dir(|_| {
                arguments.apply_config("config.toml");
            });

            match arguments.init_errors.pop_front() {
                Some(Error::Custom(string)) => {
                    assert!(
                        string.starts_with("Unable to read config file"),
                        "String predicate failed for: {string:?}"
                    );
                }
                Some(error) => assert!(false, "Unknown error: {error:?}"),
                None => assert!(
                    false,
                    "An error was expected but none was received"
                ),
            };
        }

        #[test]
        fn fails_silently_on_incorrect_config() {
            let mut arguments = arguments_with_config("configs/non_toml");

            match arguments.init_errors.pop_front() {
                Some(Error::Custom(string)) => {
                    assert!(
                        string.starts_with("Unable to parse config file"),
                        "String predicate failed for: {string:?}"
                    );
                }
                Some(error) => assert!(false, "Unknown error: {error:?}"),
                None => assert!(
                    false,
                    "An error was expected but none was received"
                ),
            };
        }

        #[test]
        fn empty() {
            let arguments = arguments_with_config("configs/empty.toml");

            assert!(arguments.init_errors.is_empty());
            assert_eq!(false, arguments.time());
            assert_eq!(DEFAULT_DATE_FORMAT, arguments.default_date_format);
            assert_eq!(
                DEFAULT_DATE_TIME_FORMAT,
                arguments.default_date_time_format
            );
            assert_eq!(false, arguments.today());
            assert!(matches!(arguments.metadata(), Metadata::None));
            assert!(arguments.patterns.is_none());
        }

        #[test]
        fn time_non_bool() {
            let arguments = arguments_with_config("configs/time/non_bool.toml");

            assert!(arguments.init_errors.is_empty());
            assert_eq!(false, arguments.time());
        }

        #[test]
        fn time() {
            let arguments = arguments_with_config("configs/time/true.toml");

            assert!(arguments.init_errors.is_empty());
            assert_eq!(true, arguments.time());
        }

        #[test]
        fn different_config() {
            let arguments = arguments_with_config("configs/different.toml");

            assert!(arguments.init_errors.is_empty());
            assert_eq!(true, arguments.time());
            assert_eq!("%m-%d %Y", arguments.default_date_format);
            assert_eq!("%m-%d %Hh%Mm%S %Y", arguments.default_date_time_format);
            assert_eq!(true, arguments.today());
            assert!(matches!(arguments.metadata(), Metadata::Both));
            assert_eq!(2, arguments.patterns.unwrap().len());
        }
    }
}
