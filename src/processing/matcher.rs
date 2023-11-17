use crate::replacement::Replacement;
use crate::matcher::Matcher as MatcherTrait;

use std::path::Path;

#[derive(Clone)]
pub struct Matcher {
    confirmed: bool,
    ignored: bool,
    matcher: Box<dyn MatcherTrait>,
}

impl Matcher {
    #[allow(clippy::borrowed_box)]
    pub fn from(matcher: &Box<dyn MatcherTrait>) -> Matcher {
        Self {
            confirmed: false,
            ignored: false,
            matcher: matcher.clone(),
        }
    }

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
