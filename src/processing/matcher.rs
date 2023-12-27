use crate::matcher::Matcher;
use crate::replacement::Replacement;

use std::path::Path;

#[derive(Clone)]
pub struct ProcessingMatcher {
    confirmed: bool,
    ignored: bool,
    matcher: Box<dyn Matcher>,
}

impl From<&Box<dyn Matcher>> for ProcessingMatcher {
    fn from(matcher: &Box<dyn Matcher>) -> Self {
        Self {
            confirmed: false,
            ignored: false,
            matcher: matcher.clone(),
        }
    }
}

impl ProcessingMatcher {
    pub fn check(&self, path: &Path) -> Option<Replacement> {
        self.matcher.check(path)
    }

    /// Check if the matcher needs confirmation
    ///
    /// Can we directly used the Replacement given by check or should we ask
    /// for confirmation?
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }

    /// Mark a matcher as confirmed
    pub fn confirm(&mut self) {
        self.confirmed = true;
    }

    /// Check if the matcher should be ignored
    pub fn ignored(&self) -> bool {
        self.ignored
    }

    /// Mark the matcher as ignored
    pub fn ignore(&mut self) {
        self.ignored = true;
    }
}
