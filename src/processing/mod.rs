use crate::application::{Application, Confirmation};
use crate::replacement::Replacement;
use crate::reporter::Reporter;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

mod matcher;
pub use matcher::Matcher;

use std::path::{Path, PathBuf};

pub struct Processing<'a> {
    app: &'a Application,
    matchers: Vec<Matcher>,
}

impl<'a> Processing<'a> {
    pub fn new(app: &'a Application) -> Processing<'a> {
        Self {
            app,
            matchers: app.matchers.iter().map(Matcher::from).collect(),
        }
    }

    pub fn run(&mut self, paths: &Vec<PathBuf>) -> Result<()> {
        self.app.count(paths.len());

        for path in paths {
            self.app.processing(path);

            match self
                .prefix_if_possible(path)
                .and_then(|replacement| replacement.execute())
            {
                Ok(replacement) => {
                    self.app.processing_ok(path, &replacement);
                }
                Err(Error::Abort) => return Err(Error::Abort),
                Err(error) => {
                    self.app.processing_err(path, &error);
                }
            }
        }

        Ok(())
    }

    pub fn prefix_if_possible(&mut self, path: &Path) -> Result<Replacement> {
        if !path.try_exists().unwrap() {
            return Err(Error::not_found(path));
        }

        let app: &Application = self.app;

        for matcher in self.matchers_mut() {
            if let Some(replacement) = matcher.check(path) {
                if matcher.confirmed() {
                    return Ok(replacement);
                }
                match app.confirm(path, &replacement) {
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

        Err(Error::no_match(path))
    }

    /// Return all non-ignored matchers
    fn matchers_mut(&mut self) -> impl Iterator<Item = &mut Matcher> + '_ {
        self.matchers
            .iter_mut()
            .filter(|matcher| !matcher.ignored())
    }
}
