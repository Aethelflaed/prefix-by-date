use crate::processing::{PathInfo, Result};
use crate::reporter::Reporter;
use crate::state::State;
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
                Ok(replacement) => {
                    self.state.reporter.processing_ok(
                        path,
                        replacement.result(self.state).as_str(),
                    );
                }
                Err(error) => {
                    self.state.reporter.processing_err(path, &error);
                }
            }
        }

        Ok(())
    }
}
