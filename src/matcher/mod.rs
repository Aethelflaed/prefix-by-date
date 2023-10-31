use crate::replacement::Replacement;
use chrono::{DateTime, Local};
use std::any::Any;
use std::boxed::Box;

pub mod pattern;
pub use pattern::Pattern;

/// Match a file to be renamed
pub trait Matcher {
    /// Check if the given file_name should be replaced by the matcher and
    /// if so, return the appropriate Replacement
    fn check(&self, file_name: &str) -> Option<Replacement>;

    /// Name of the matcher
    fn name(&self) -> &str;
    /// Delimiter to place between the matched elements
    fn delimiter(&self) -> &str;

    /// Convenience function to allow downcast_ref
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone)]
pub struct PredeterminedDate {
    pub date_time: DateTime<Local>,
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

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn predetermined_date_matcher() {
        let matcher = PredeterminedDate {
            date_time: Local::now(),
        };

        let replacement = matcher.check("foo").unwrap();
        assert_eq!(String::from("foo"), replacement.rest);
        assert_eq!(matcher.date_time, replacement.date_time);

        let replacement_matcher = replacement
            .matcher
            .as_any()
            .downcast_ref::<PredeterminedDate>()
            .unwrap();
        assert_eq!(matcher.date_time, replacement_matcher.date_time);
    }
}
