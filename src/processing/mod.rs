mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

mod path_info;
pub use path_info::PathInfo;

use crate::reporter::Reporter;
use crate::state::State;
use std::path::PathBuf;

pub struct Processing<'a> {
    state: &'a State,
    paths: &'a Vec<PathBuf>,
}

impl<'a> Processing<'a> {
    pub fn new(state: &'a State, paths: &'a Vec<PathBuf>) -> Processing<'a> {
        state.count(paths.len());
        Self { state, paths }
    }

    pub fn run(&self) -> Result<()> {
        for path in self.paths {
            self.state.processing(path);

            let path_info = PathInfo {
                state: self.state,
                path,
            };

            match path_info.prefix_if_possible() {
                Ok(replacement) => {
                    self.state
                        .processing_ok(path, replacement.result().as_str());
                }
                Err(error) => {
                    self.state.processing_err(path, &error);
                }
            }
        }

        Ok(())
    }
}
