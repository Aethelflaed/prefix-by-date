mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

use crate::cli::Cli;
use crate::matcher::{Matcher, Pattern, PredeterminedDate};
use crate::processing;
use crate::replacement::Replacement;
use crate::reporter::{Log, Reporter};

use std::boxed::Box;
use std::path::{Path, PathBuf};

use toml::Table;

type LogResult = std::result::Result<(), log::SetLoggerError>;

#[cfg(feature = "cli")]
use indicatif::{MultiProgress, ProgressBar};

pub struct Application {
    pub matchers: Vec<Box<dyn Matcher>>,
    pub reporters: Vec<Box<dyn Reporter>>,
    pub cli: Cli,
    #[cfg(feature = "cli")]
    bar: Option<ProgressBar>,
    #[cfg(feature = "cli")]
    multi_progress: MultiProgress,
}

#[allow(dead_code)]
pub enum Confirmation {
    Accept,
    //Always,
    //Refuse,
    //Ignore,
    Replace(Replacement),
}

#[allow(clippy::derivable_impls)]
impl Default for Application {
    #[cfg(not(feature = "cli"))]
    fn default() -> Self {
        Self {
            matchers: Vec::<Box<dyn Matcher>>::default(),
            reporters: Vec::<Box<dyn Reporter>>::default(),
            cli: Cli::default(),
        }
    }

    #[cfg(feature = "cli")]
    fn default() -> Self {
        // We need a hidden ProgressDrawTarget for the tests if we don't
        // want to polute the output
        #[cfg(test)]
        let multi_progress = MultiProgress::with_draw_target(
            indicatif::ProgressDrawTarget::hidden(),
        );
        #[cfg(not(test))]
        let multi_progress = MultiProgress::new();

        Self {
            matchers: Vec::<Box<dyn Matcher>>::default(),
            reporters: Vec::<Box<dyn Reporter>>::default(),
            cli: Cli::default(),
            multi_progress,
            bar: None,
        }
    }
}

#[cfg(feature = "cli")]
impl Drop for Application {
    fn drop(&mut self) {
        if let Some(bar) = &self.bar {
            bar.finish();
            self.multi_progress.remove(bar);
        }
    }
}

impl Application {
    pub fn new() -> Result<Self> {
        let mut app = Self::default();
        app.setup_log()?;

        Ok(app)
    }

    pub fn setup(&mut self, cli: &Cli) -> Result<()> {
        log::set_max_level(cli.verbose.log_level_filter());

        let mut format = "%Y-%m-%d";
        if cli.time {
            log::debug!("Prefix by date and time");
            format = "%Y-%m-%d %Hh%Mm%S";
        }

        if cli.today {
            log::debug!("Prefix by today's date");

            self.matchers.push(Box::new(PredeterminedDate::new(format)));
        }

        self.read_config(format)?;
        self.add_reporter(Box::<Log>::default());
        self.after_setup(cli)?;

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

    #[cfg(not(feature = "cli"))]
    pub fn confirm(
        &self,
        _path: &Path,
        _replacement: &Replacement,
    ) -> Confirmation {
        Confirmation::Accept
    }

    #[cfg(feature = "cli")]
    pub fn confirm(
        &self,
        path: &Path,
        replacement: &Replacement,
    ) -> Confirmation {
        use dialoguer::FuzzySelect;

        println!("{} will be renamed into {}", path.display(), replacement.result());

        let items = vec![
            "Yes, accept the rename and continue",
            "Always accept similar rename and continue",
            "Refuse the rename and continue",
            "Ignore all similar rename and continue",
            "Quit now, refusing this rename",
            "View other possibilities",
            "Customize the rename"
        ];

        let selection = FuzzySelect::new()
            .with_prompt("What do you want to do?")
            .items(&items)
            .interact()
            .unwrap();

        match selection {
            0 => { return Confirmation::Accept },
            _ => todo!(),
        }
        println!("You chose: {}", items[selection]);

        Confirmation::Accept
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
            self.setup_env_logger()
        }
    }

    #[cfg(feature = "cli")]
    fn setup_env_logger(&mut self) -> LogResult {
        use env_logger::{Builder, Env};
        use indicatif_log_bridge::LogWrapper;

        let env = Env::new()
            .filter(format!("{}_LOG", env!("CARGO_PKG_NAME")))
            .write_style(format!("{}_LOG_STYLE", env!("CARGO_PKG_NAME")));

        let logger = Builder::new()
            .filter_level(log::LevelFilter::Trace)
            .parse_env(env)
            .build();

        LogWrapper::new(self.multi_progress.clone(), logger).try_init()
    }

    #[cfg(not(feature = "cli"))]
    fn setup_env_logger(&mut self) -> LogResult {
        use env_logger::{Builder, Env};

        let env = Env::new()
            .filter(format!("{}_LOG", env!("CARGO_PKG_NAME")))
            .write_style(format!("{}_LOG_STYLE", env!("CARGO_PKG_NAME")));

        Builder::new()
            .filter_level(log::LevelFilter::Trace)
            .parse_env(env)
            .try_init()
    }

    #[cfg(feature = "cli")]
    fn after_setup(&mut self, cli: &Cli) -> Result<()> {
        self.bar = Some(
            self.multi_progress
                .add(ProgressBar::new(cli.paths.len() as u64)),
        );

        Ok(())
    }

    #[cfg(not(feature = "cli"))]
    fn after_setup(&mut self, _cli: &Cli) -> Result<()> {
        Ok(())
    }

    #[cfg(feature = "cli")]
    fn after_proces(&self, _path: &Path) {
        if let Some(bar) = &self.bar {
            bar.inc(1);
        }
    }

    #[cfg(not(feature = "cli"))]
    fn after_proces(&self, _path: &Path) {}
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
        self.after_proces(path);

        for reporter in &self.reporters {
            reporter.processing_err(path, error);
        }
    }

    fn processing_ok(&self, path: &Path, new_name: &str) {
        self.after_proces(path);

        for reporter in &self.reporters {
            reporter.processing_ok(path, new_name);
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
