use crate::matcher::Matcher;
use crate::replacement::Replacement;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

mod matcher;
pub use matcher::ProcessingMatcher;

mod log_reporter;
mod notif_reporter;

use std::boxed::Box;
use std::path::{Path, PathBuf};

pub struct Processing<'a, T>
where
    T: Communication,
{
    matchers: Vec<ProcessingMatcher<'a>>,
    paths: &'a [PathBuf],
    interface: &'a T,
    reporters: Vec<Box<dyn Reporter>>,
}

pub trait Reporter {
    /// Report the total count of elements about to be processed
    fn setup(&self, count: usize);
    /// Start processing this path
    fn processing(&self, path: &Path);
    /// Processing went well and ended-up with this replacement
    fn processing_ok(&self, replacement: &Replacement);
    /// Processing encountered this error
    fn processing_err(&self, path: &Path, error: &Error);
}

pub trait Communication: Reporter {
    /// Whenever a matcher finds a replacement, confirm it
    fn confirm(&self, replacement: &Replacement) -> Confirmation;
    /// If no match is found, attempt to rescue the Error::NoMatch
    fn rescue(&self, error: Error) -> Result<Replacement>;
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Confirmation {
    Accept,
    Always,
    Skip,
    Refuse,
    Ignore,
    Abort,
    Replace(Replacement),
}

impl PartialEq for Confirmation {
    fn eq(&self, other: &Confirmation) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl<'a, T> Processing<'a, T>
where
    T: Communication,
{
    pub fn new(
        interface: &'a T,
        matchers: &'a [Box<dyn Matcher>],
        paths: &'a [PathBuf],
    ) -> Self {
        Self {
            matchers: matchers.iter().map(From::<_>::from).collect(),
            paths,
            interface,
            reporters: vec![
                Box::<log_reporter::LogReporter>::default(),
                #[cfg(feature = "notif")]
                Box::<notif_reporter::NotifReporter>::default(),
            ],
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.report_setup(self.paths.len());

        for path in self.paths {
            self.report_processing(path);

            match self
                .prefix_if_possible(path)
                .and_then(|replacement| replacement.execute())
            {
                Ok(replacement) => {
                    self.report_processing_ok(&replacement);
                }
                Err(error) => {
                    self.report_processing_err(path, &error);

                    if let Error::Abort = error {
                        return Err(error);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn prefix_if_possible(&mut self, path: &Path) -> Result<Replacement> {
        if !path.try_exists().unwrap() {
            return Err(Error::not_found(path));
        }

        // Get an immutable ref
        let interface: &T = self.interface;

        let mut found = false;

        for matcher in self
            .matchers
            .iter_mut()
            .filter(|matcher| !matcher.ignored())
        {
            if let Some(replacement) = matcher.check(path) {
                found = true;
                if matcher.confirmed() {
                    return Ok(replacement);
                }
                match interface.confirm(&replacement) {
                    Confirmation::Accept => return Ok(replacement),
                    Confirmation::Always => {
                        matcher.confirm();
                        return Ok(replacement);
                    }
                    Confirmation::Skip => {
                        return Err(Error::Skip(path.to_path_buf()));
                    }
                    Confirmation::Refuse => {}
                    Confirmation::Ignore => {
                        matcher.ignore();
                    }
                    Confirmation::Abort => {
                        return Err(Error::Abort);
                    }
                    Confirmation::Replace(replacement) => {
                        return Ok(replacement)
                    }
                };
            }
        }

        if found {
            Err(Error::no_match(path))
        } else {
            interface.rescue(Error::no_match(path))
        }
    }

    fn report_setup(&self, count: usize) {
        for reporter in &self.reporters {
            reporter.setup(count);
        }

        self.interface.setup(count);
    }
    fn report_processing(&self, path: &Path) {
        for reporter in &self.reporters {
            reporter.processing(path);
        }

        self.interface.processing(path);
    }
    fn report_processing_ok(&self, replacement: &Replacement) {
        for reporter in &self.reporters {
            reporter.processing_ok(replacement);
        }

        self.interface.processing_ok(replacement);
    }
    fn report_processing_err(&self, path: &Path, error: &Error) {
        for reporter in &self.reporters {
            reporter.processing_err(path, error);
        }

        self.interface.processing_err(path, error);
    }
}
