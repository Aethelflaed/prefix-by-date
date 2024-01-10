use crate::replacement::Replacement;

use std::path::Path;

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
    fn check(&self, path: &Path) -> Option<Replacement>;

    /// Name of the matcher
    fn name(&self) -> &str;
    /// Delimiter to place between the matched elements
    fn delimiter(&self) -> &str;
    /// Format to use for the date
    fn date_format(&self) -> &str;
}

dyn_clone::clone_trait_object!(Matcher);
