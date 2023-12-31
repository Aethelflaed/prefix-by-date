use crate::replacement::Replacement;

use std::path::Path;

use chrono::{DateTime, Local};
use dyn_clone::DynClone;

pub mod pattern;
pub use pattern::Pattern;

/// Match a file to be renamed
pub trait Matcher: DynClone + Send {
    /// Check if the given path should be replaced by the matcher and
    /// if so, return the appropriate Replacement
    fn check(&self, path: &Path) -> Option<Replacement>;

    /// Name of the matcher
    fn name(&self) -> &str;
    /// Delimiter to place between the matched elements
    fn delimiter(&self) -> &str;
    /// Format to use for the date
    fn date_format(&self) -> &str;
    /// Does this matcher handle time as well as date?
    fn time(&self) -> bool;
}

dyn_clone::clone_trait_object!(Matcher);

#[derive(Clone)]
pub struct PredeterminedDate {
    pub date_time: DateTime<Local>,
    pub format: String,
    pub time: bool,
}

impl Default for PredeterminedDate {
    fn default() -> Self {
        Self {
            date_time: Local::now(),
            format: String::from("%Y-%m-%d"),
            time: false,
        }
    }
}

impl Matcher for PredeterminedDate {
    fn check(&self, path: &Path) -> Option<Replacement> {
        let mut replacement = Replacement::try_from(path).ok()?;
        replacement.new_file_stem = format!(
            "{} {}",
            self.date_time.format(self.date_format()),
            replacement.file_stem,
        );

        Some(replacement)
    }

    fn name(&self) -> &str {
        "Predetermined date"
    }

    fn delimiter(&self) -> &str {
        " "
    }

    fn date_format(&self) -> &str {
        self.format.as_str()
    }

    fn time(&self) -> bool {
        self.time
    }
}

impl PredeterminedDate {
    pub fn new(format: &str, time: bool) -> Self {
        Self {
            format: format.into(),
            time,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    use chrono::TimeZone;
    use std::path::PathBuf;

    fn date_time(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        min: u32,
        sec: u32,
    ) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, min, sec)
            .earliest()
            .unwrap()
    }

    fn date(year: i32, month: u32, day: u32) -> DateTime<Local> {
        date_time(year, month, day, 0, 0, 0)
    }

    #[test]
    fn predetermined_date_matcher() {
        let matcher = PredeterminedDate {
            date_time: date(2023, 10, 31),
            format: String::from("%Y-%m-%d %Hh%Mm"),
            time: false,
        };

        let replacement = matcher.check(&PathBuf::from("foo.bar")).unwrap();
        assert_eq!(
            PathBuf::from("2023-10-31 00h00m foo.bar"),
            replacement.new_path()
        );
    }
}
