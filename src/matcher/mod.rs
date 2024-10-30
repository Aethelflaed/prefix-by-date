use crate::replacement::Replacement;

use std::fmt;
use std::path::Path;

use chrono::{DateTime, Local};
use dyn_clone::DynClone;

pub mod predetermined_date;
pub use predetermined_date::PredeterminedDate;

pub mod pattern;
pub use pattern::Pattern;

pub mod metadata;
pub use metadata::Metadata;

/// Match a file to be renamed
pub trait Matcher: DynClone + Send {
    /// Check if the given path should be replaced by the matcher and
    /// if so, return the appropriate Replacement
    fn check(&self, path: &Path) -> Option<Replacement> {
        let mut replacement = Replacement::try_from(path).ok()?;
        let (name, date_time) = self.determine(&replacement)?;

        replacement.new_file_stem = format!(
            "{}{}{}",
            date_time.format(self.date_format()),
            self.delimiter(),
            name
        );

        Some(replacement)
    }

    /// Determine the name and date-time to use
    ///
    /// The whole &Replacement is passed so you can access the path() if needed,
    /// or directly the file_stem
    fn determine(
        &self,
        replacement: &Replacement,
    ) -> Option<(String, DateTime<Local>)>;

    /// Name of the matcher
    fn name(&self) -> &str;
    /// Delimiter to place between the matched elements
    fn delimiter(&self) -> &str;
    /// Format to use for the date
    fn date_format(&self) -> &str;

    /// Indicates if a replacement produced by this matcher can be accepted
    /// without user confirmation or not.
    fn auto_accept(&self) -> bool;
}

impl fmt::Debug for dyn Matcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.name())
    }
}

dyn_clone::clone_trait_object!(Matcher);
