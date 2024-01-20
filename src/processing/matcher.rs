use crate::matcher::Matcher;
use crate::replacement::Replacement;

use std::path::Path;

pub struct ProcessingMatcher<'a> {
    confirmed: bool,
    ignored: bool,
    matcher: &'a dyn Matcher,
}

impl<'a> From<&'a Box<dyn Matcher>> for ProcessingMatcher<'a> {
    fn from(matcher: &'a Box<dyn Matcher>) -> Self {
        Self {
            confirmed: matcher.auto_accept(),
            ignored: false,
            matcher: matcher.as_ref(),
        }
    }
}

impl<'a> ProcessingMatcher<'a> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::test;

    use std::path::PathBuf;

    use crate::matcher::{Pattern, PredeterminedDate};

    #[test]
    fn auto_accept_matcher_is_confirmed() {
        let matcher: Box<dyn Matcher> = Box::<PredeterminedDate>::default();
        let processing_matcher = ProcessingMatcher::from(&matcher);

        assert!(processing_matcher.confirmed());
    }

    #[test]
    fn confirm() {
        let matcher: Box<dyn Matcher> = Box::<Pattern>::default();
        let mut processing_matcher = ProcessingMatcher::from(&matcher);

        assert!(!processing_matcher.confirmed());
        processing_matcher.confirm();
        assert!(processing_matcher.confirmed());
    }

    #[test]
    fn ignore() {
        let matcher: Box<dyn Matcher> = Box::<Pattern>::default();
        let mut processing_matcher = ProcessingMatcher::from(&matcher);

        assert!(!processing_matcher.ignored());
        processing_matcher.ignore();
        assert!(processing_matcher.ignored());
    }

    #[test]
    fn check() {
        let matcher: Box<dyn Matcher> = Box::<Pattern>::default();
        let processing_matcher = ProcessingMatcher::from(&matcher);
        let path = PathBuf::from("foo");

        assert!(processing_matcher.check(&path).is_none());
    }
}
