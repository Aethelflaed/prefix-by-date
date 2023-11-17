use crate::replacement::Replacement;

use std::boxed::Box;
use std::cell::Cell;

use chrono::{DateTime, Local};
use dyn_clone::DynClone;

pub mod pattern;
pub use pattern::Pattern;

/// Match a file to be renamed
pub trait Matcher: DynClone {
    /// Check if the given file_name should be replaced by the matcher and
    /// if so, return the appropriate Replacement
    fn check(&self, file_name: &str) -> Option<Replacement>;

    /// Name of the matcher
    fn name(&self) -> &str;
    /// Delimiter to place between the matched elements
    fn delimiter(&self) -> &str;
    /// Format to use for the date
    fn date_format(&self) -> &str;

    /// Check if the matcher needs confirmation
    ///
    /// Can we directly used the Replacement given by check or should we ask
    /// for confirmation?
    fn confirmed(&self) -> bool;
    /// Mark a matcher as confirmed
    fn confirm(&self);

    /// Check if the matcher should be ignored
    fn ignored(&self) -> bool;
    /// Mark the matcher as ignored
    fn ignore(&self);
}

dyn_clone::clone_trait_object!(Matcher);

#[derive(Clone)]
pub struct PredeterminedDate {
    pub date_time: DateTime<Local>,
    pub format: String,
    confirmed: Cell<bool>,
    ignored: Cell<bool>,
}

impl Default for PredeterminedDate {
    fn default() -> Self {
        Self {
            date_time: Local::now(),
            format: String::from("%Y-%m-%d"),
            confirmed: Cell::<bool>::new(false),
            ignored: Cell::<bool>::new(false),
        }
    }
}

impl Matcher for PredeterminedDate {
    fn check(&self, file_name: &str) -> Option<Replacement> {
        Some(Replacement {
            matcher: Box::new(self.clone()),
            date_time: self.date_time,
            rest: file_name.into(),
        })
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

    fn confirmed(&self) -> bool {
        self.confirmed.get()
    }

    fn confirm(&self) {
        self.confirmed.set(true);
    }

    fn ignored(&self) -> bool {
        self.ignored.get()
    }

    fn ignore(&self) {
        self.ignored.set(true);
    }
}

impl PredeterminedDate {
    pub fn new(format: &str) -> Self {
        Self {
            format: format.into(),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use pretty_assertions::assert_eq;

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
        };

        let replacement = matcher.check("foo").unwrap();
        assert_eq!(String::from("2023-10-31 00h00m foo"), replacement.result());
    }
}
