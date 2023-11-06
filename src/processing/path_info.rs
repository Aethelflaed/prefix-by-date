use crate::context::{Confirmation, Context};
use crate::processing::{Error, Result};
use crate::replacement::Replacement;
use std::path::PathBuf;

pub struct PathInfo<'a> {
    pub context: &'a Context,
    pub path: &'a PathBuf,
}

impl<'a> PathInfo<'a> {
    pub fn prefix_if_possible(&self) -> Result<Replacement> {
        if !self.path.try_exists().unwrap() {
            return Err(Error::not_found(self.path));
        }

        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        for matcher in &self.context.matchers {
            if let Some(replacement) = matcher.check(file_name) {
                return match self.context.confirm(self.path, &replacement) {
                    Confirmation::Replace(replacement) => {
                        match self.rename(replacement.result().as_str()) {
                            Ok(()) => Ok(replacement),
                            Err(error) => Err(error),
                        }
                    }
                    Confirmation::Accept => {
                        match self.rename(replacement.result().as_str()) {
                            Ok(()) => Ok(replacement),
                            Err(error) => Err(error),
                        }
                    }
                };
            }
        }

        Err(Error::no_match(self.path))
    }

    fn rename(&self, new_name: &str) -> Result<()> {
        let mut new_path = self.path.clone();
        new_path.pop();
        new_path.push(new_name);

        std::fs::rename(self.path, new_path)?;

        Ok(())
    }
}
