use crate::processing::{Error, ErrorKind, Result};
use crate::replacement::Replacement;
use crate::state::State;
use std::path::PathBuf;

pub struct PathInfo<'a> {
    pub state: &'a State,
    pub path: &'a PathBuf,
}

impl<'a> PathInfo<'a> {
    pub fn prefix_if_possible(&self) -> Result<Replacement> {
        if !self.path.try_exists().unwrap() {
            return Err(Error::new(ErrorKind::NotFound(self.path.clone())));
        }

        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        for matcher in &self.state.matchers {
            if let Some(replacement) = matcher.check(file_name) {
                log::debug!("Match: {}", matcher.name());

                return match self
                    .rename(replacement.result(self.state).as_str())
                {
                    Ok(()) => Ok(replacement),
                    Err(error) => Err(error),
                };
            }
        }

        Err(Error::new(ErrorKind::NoMatch(self.path.clone())))
    }

    fn rename(&self, new_name: &str) -> Result<()> {
        let mut new_path = self.path.clone();
        new_path.pop();
        new_path.push(new_name);

        std::fs::rename(self.path, new_path)?;

        Ok(())
    }
}
