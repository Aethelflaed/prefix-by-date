use crate::application::cli::{Cli, Interactive, Metadata};

use std::path::PathBuf;

#[derive(Debug)]
pub struct Arguments {
    /// Command-line interface arguments
    cli: Cli,

    time: bool,
    default_date_format: String,
    default_date_time_format: String,

    today: bool,
    metadata: Metadata,

    patterns: toml::Table,
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
            patterns: toml::Table::default(),
        }
    }
}

pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d";
pub const DEFAULT_DATE_TIME_FORMAT: &str = "%Y-%m-%d %Hh%Mm%S";

impl Arguments {
    pub fn parse() -> Self {
        use clap::Parser;

        Self {
            cli: Cli::parse(),
            ..Arguments::default()
        }
        .apply_config()
        .apply_cli()
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

    pub fn patterns(&self) -> &toml::Table {
        &self.patterns
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.cli.paths
    }

    fn apply_cli(mut self) -> Self {
        if let Some(time) = self.cli.time() {
            self.time = time;
        }

        if let Some(metadata) = self.cli.metadata {
            self.metadata = metadata;
        }

        self.today = self.cli.today;

        self
    }

    fn apply_config(mut self) -> Self {
        use toml::{Table, Value};

        let dir = self.cli.config.take().unwrap_or_else(config_home);
        let path = dir.join("config.toml");

        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(config_table) = content.parse::<Table>() {
                if let Some(value) =
                    config_table.get("time").and_then(Value::as_bool)
                {
                    self.time = value;
                }

                if let Some(table) =
                    config_table.get("default_format").and_then(Value::as_table)
                {
                    if let Some(format) =
                        table.get("date").and_then(Value::as_str)
                    {
                        self.default_date_format = format.to_string();
                    }
                    if let Some(format) =
                        table.get("date_time").and_then(Value::as_str)
                    {
                        self.default_date_time_format = format.to_string();
                    }
                }

                if let Some(matchers) =
                    config_table.get("matchers").and_then(Value::as_table)
                {
                    if let Some(patterns) =
                        matchers.get("patterns").and_then(Value::as_table)
                    {
                        self.patterns = patterns.clone();
                    }

                    if let Some(predet) = matchers
                        .get("predetermined_date")
                        .and_then(Value::as_table)
                    {
                        if let Some(today) =
                            predet.get("today").and_then(Value::as_bool)
                        {
                            self.today = today;
                        }
                    }

                    if let Some(metadata) =
                        matchers.get("metadata").and_then(Value::as_table)
                    {
                        let created =
                            metadata.get("created").and_then(Value::as_bool);
                        let modified =
                            metadata.get("modified").and_then(Value::as_bool);

                        #[cfg(test)]
                        assert!(!matches!(self.metadata, Metadata::Both));

                        match (created, modified) {
                            (Some(false), Some(false)) => {
                                self.metadata = Metadata::None
                            }
                            (Some(false), _) => {
                                self.metadata = Metadata::Modified
                            }
                            (_, Some(false)) => {
                                self.metadata = Metadata::Created
                            }
                            (_, _) => {}
                        };
                    }
                }
            }
        }

        self
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
    use pretty_assertions::assert_eq;
    use temp_env::with_var;

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
}
