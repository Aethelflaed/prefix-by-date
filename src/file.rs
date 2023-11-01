use crate::reporter::Reporter;
use crate::state::State;
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;

pub struct Processing<'a> {
    state: &'a State,
    paths: &'a Vec<PathBuf>,
}

impl<'a> Processing<'a> {
    pub fn new(state: &'a State, paths: &'a Vec<PathBuf>) -> Processing<'a> {
        state.reporter.count(paths.len());
        Processing { state, paths }
    }

    pub fn run(&self) -> Result<()> {
        for path in self.paths {
            self.state.reporter.processing(path);

            let path_info = PathInfo {
                state: self.state,
                path,
            };

            match path_info.prefix_if_possible() {
                Ok(_) => {
                    self.state.reporter.processing_ok(path);
                }
                Err(error) => {
                    self.state.reporter.processing_err(path, &error);
                }
            }
        }

        Ok(())
    }
}

pub struct PathInfo<'a> {
    pub state: &'a State,
    pub path: &'a PathBuf,
}

impl<'a> PathInfo<'a> {
    pub fn prefix_if_possible(&self) -> Result<()> {
        if !self.path.try_exists().unwrap() {
            return Err(Error::new(ErrorKind::NotFound, "Path does not exist"));
        }

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
