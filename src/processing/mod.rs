use crate::application::{Application, Confirmation};
use crate::matcher::Matcher;
use crate::replacement::Replacement;
use crate::reporter::Reporter;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

use std::path::{Path, PathBuf};

pub struct Processing<'a> {
    app: &'a Application,
    matchers: Vec<Box<dyn Matcher>>,
}

impl<'a> Processing<'a> {
    pub fn new(app: &'a Application) -> Processing<'a> {
        Self {
            app,
            matchers: app.matchers.clone(),
        }
    }

    pub fn run(&self, paths: &Vec<PathBuf>) -> Result<()> {
        self.app.count(paths.len());

        for path in paths {
            self.app.processing(path);

            match self.prefix_if_possible(path).and_then(|replacement| {
                self.rename(path, replacement.result().as_str())
                    .map(|()| replacement)
            }) {
                Ok(replacement) => {
                    self.app.processing_ok(path, replacement.result().as_str());
                }
                Err(error) => {
                    self.app.processing_err(path, &error);

                    if let Error::Abort = error {
                        return Err(error);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn prefix_if_possible(&self, path: &Path) -> Result<Replacement> {
        if !path.try_exists().unwrap() {
            return Err(Error::not_found(path));
        }

        let file_name = path.file_name().unwrap().to_str().unwrap();

        for matcher in self.matchers() {
            if let Some(replacement) = matcher.check(file_name) {
                if matcher.confirmed() {
                    return Ok(replacement);
                }
                match self.app.confirm(path, &replacement) {
                    Confirmation::Replace(replacement) => {
                        return Ok(replacement)
                    }
                    Confirmation::Accept => return Ok(replacement),
                    Confirmation::Always => {
                        matcher.confirm();
                        return Ok(replacement);
                    }
                    Confirmation::Refuse => {}
                    Confirmation::Ignore => {
                        matcher.ignore();
                    }
                    Confirmation::Abort => {
                        return Err(Error::Abort);
                    }
                };
            }
        }

        Err(Error::no_match(path))
    }

    /// Return all non-ignored matchers
    fn matchers(&self) -> impl Iterator<Item = &Box<dyn Matcher>> + '_ {
        self.matchers.iter().filter(|matcher| !matcher.ignored())
    }

    fn rename(&self, path: &PathBuf, new_name: &str) -> Result<()> {
        let mut new_path = path.clone();
        new_path.pop();
        new_path.push(new_name);

        std::fs::rename(path, new_path)?;

        Ok(())
    }
}
