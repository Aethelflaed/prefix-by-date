use crate::application::DEFAULT_DATE_FORMAT;
use crate::matcher::Matcher;
use crate::replacement::Replacement;

use chrono::{DateTime, Local};

#[derive(Default, Clone, Copy)]
enum When {
    #[default]
    Today,
}

impl When {
    fn name(&self) -> &'static str {
        match self {
            When::Today => TODAY,
        }
    }
}

pub const TODAY: &str = "predetermined_date today";

#[derive(Clone)]
pub struct PredeterminedDate {
    when: When,
    date_time: DateTime<Local>,
    format: String,
}

impl Default for PredeterminedDate {
    fn default() -> Self {
        Self {
            when: When::default(),
            date_time: Local::now(),
            format: String::from(DEFAULT_DATE_FORMAT),
        }
    }
}

impl PredeterminedDate {
    pub fn new(format: &str) -> Self {
        Self {
            format: format.to_string(),
            ..Self::default()
        }
    }
}

impl Matcher for PredeterminedDate {
    fn determine(
        &self,
        replacement: &Replacement,
    ) -> Option<(String, DateTime<Local>)> {
        Some((replacement.file_stem.clone(), self.date_time))
    }

    fn name(&self) -> &str {
        self.when.name()
    }

    fn delimiter(&self) -> &str {
        " "
    }

    fn date_format(&self) -> &str {
        self.format.as_str()
    }

    fn auto_accept(&self) -> bool {
        true
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
        use crate::application::DEFAULT_DATE_TIME_FORMAT;

        let matcher = PredeterminedDate {
            date_time: date(2023, 10, 31),
            format: String::from(DEFAULT_DATE_TIME_FORMAT),
            ..PredeterminedDate::default()
        };

        let replacement = matcher.check(&PathBuf::from("foo.bar")).unwrap();
        assert_eq!(
            PathBuf::from("2023-10-31 00h00m00 foo.bar"),
            replacement.new_path()
        );
    }
}
