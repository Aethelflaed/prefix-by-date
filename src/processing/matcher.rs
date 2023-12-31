use crate::matcher::Matcher;
use crate::replacement::Replacement;

use std::path::Path;

#[derive(Clone)]
pub struct ProcessingMatcher {
    confirmed: bool,
    ignored: bool,
    matcher: Box<dyn Matcher>,
}

impl From<Box<dyn Matcher>> for ProcessingMatcher {
    fn from(matcher: Box<dyn Matcher>) -> Self {
        Self {
            confirmed: false,
            ignored: false,
            matcher,
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    use crate::matcher::Pattern;

    fn matcher() -> ProcessingMatcher {
        ProcessingMatcher::from(
            Box::<Pattern>::default() as Box::<dyn Matcher>
        )
    }

    #[test]
    fn confirm() {
        let mut processing_matcher = matcher();

        assert!(!processing_matcher.confirmed());
        processing_matcher.confirm();
        assert!(processing_matcher.confirmed());
    }

    #[test]
    fn ignore() {
        let mut processing_matcher = matcher();

        assert!(!processing_matcher.ignored());
        processing_matcher.ignore();
        assert!(processing_matcher.ignored());
    }

    #[test]
    fn check() {
        let processing_matcher = matcher();
        let path = PathBuf::from("foo");

        assert!(processing_matcher.check(&path).is_none());
    }
}
