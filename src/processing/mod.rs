use crate::application::{Application, Confirmation};
use crate::replacement::Replacement;
use crate::reporter::Reporter;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

use std::path::PathBuf;

pub struct Processing<'a> {
    app: &'a Application,
}

impl<'a> Processing<'a> {
    pub fn new(app: &'a Application) -> Processing<'a> {
        Self { app }
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
                }
            }
        }

        Ok(())
    }

    pub fn prefix_if_possible(&self, path: &PathBuf) -> Result<Replacement> {
        if !path.try_exists().unwrap() {
            return Err(Error::not_found(path));
        }

        let file_name = path.file_name().unwrap().to_str().unwrap();

        for matcher in &self.app.matchers {
            if let Some(replacement) = matcher.check(file_name) {
                return match self.app.confirm(path, &replacement) {
                    Confirmation::Replace(replacement) => {
                        Ok(replacement)
                    }
                    Confirmation::Accept => {
                        Ok(replacement)
                    }
                };
            }
        }

        Err(Error::no_match(path))
    }

    fn rename(&self, path: &PathBuf, new_name: &str) -> Result<()> {
        let mut new_path = path.clone();
        new_path.pop();
        new_path.push(new_name);

        std::fs::rename(path, new_path)?;

        Ok(())
    }
}
