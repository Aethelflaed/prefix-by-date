use crate::state::State;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;

pub struct Processing<'a> {
    state: &'a State,
    paths: &'a Vec<PathBuf>,
}

impl<'a> Processing<'a> {
    pub fn new(state: &'a State, paths: &'a Vec<PathBuf>) -> Processing<'a> {
        Processing { state, paths }
    }

    pub fn run(&self) -> Result<()> {
        for path in self.paths {
            log::info!("Checking path: {:?}", path);

            let file = File::new(self.state, path)?;
            file.prefix_if_possible()?;
        }

        Ok(())
    }
}

pub struct File<'a> {
    state: &'a State,
    path: &'a PathBuf,
}

impl<'a> File<'a> {
    pub fn new(state: &'a State, path: &'a PathBuf) -> Result<File<'a>> {
        if !path.try_exists().unwrap() {
            return Err(Error::new(ErrorKind::NotFound, "File does not exist"));
        }

        Ok(File { state, path })
    }

    pub fn prefix_if_possible(&self) -> Result<()> {
        let file_name = self.path.file_name().unwrap().to_str().unwrap();

        for matcher in &self.state.matchers {
            if let Some(replacement) = matcher.check(file_name) {
                log::debug!("Match: {}", matcher.name());

                self.rename(replacement.result(self.state).as_str())?;
            }
        }
        Ok(())
    }

    fn rename(&self, new_name: &str) -> Result<()> {
        let mut new_path = self.path.clone();
        new_path.pop();
        new_path.push(new_name);

        log::info!("Renaming: {:?} -> {:?}", self.path, new_path);

        std::fs::rename(self.path, new_path)
    }
}
