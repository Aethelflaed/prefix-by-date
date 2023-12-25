use crate::matcher::Matcher as MatcherTrait;
use crate::replacement::Replacement;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

mod matcher;
pub use matcher::Matcher;

mod log_reporter;
use log_reporter::LogReporter;

use std::boxed::Box;
use std::path::{Path, PathBuf};

pub struct Processing<'a, T>
where
    T: Communication,
{
    matchers: Vec<Matcher>,
    paths: Vec<PathBuf>,
    interface: &'a T,
    reporter: LogReporter,
}

pub trait Communication {
    /// Start processing this path
    fn processing(&self, path: &Path);
    /// Processing went well and ended-up with this replacement
    fn processing_ok(&self, replacement: &Replacement);
    /// Processing encountered this error
    fn processing_err(&self, path: &Path, error: &Error);

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

impl<'a, T> Processing<'a, T>
where
    T: Communication,
{
    pub fn new(
        interface: &'a T,
        matchers: &[Box<dyn MatcherTrait>],
        paths: &[PathBuf],
    ) -> Self {
        Self {
            matchers: matchers.iter().map(From::<_>::from).collect(),
            paths: paths.to_owned(),
            interface,
            reporter: Default::default(),
        }
    }

    pub fn run(&mut self) -> Result<()> {
        self.reporter.count(self.paths.len());

        let paths = self.paths.clone();

        for path in &paths {
            self.reporter.processing(path);
            self.interface.processing(path);

            match self
                .prefix_if_possible(path)
                .and_then(|replacement| replacement.execute())
            {
                Ok(replacement) => {
                    self.reporter.processing_ok(&replacement);
                    self.interface.processing_ok(&replacement);
                }
                Err(error) => {
                    self.reporter.processing_err(path, &error);
                    self.interface.processing_err(path, &error);

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

        for matcher in self.matchers_mut() {
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

    /// Return all non-ignored matchers
    fn matchers_mut(&mut self) -> impl Iterator<Item = &mut Matcher> + '_ {
        self.matchers
            .iter_mut()
            .filter(|matcher| !matcher.ignored())
    }
}
