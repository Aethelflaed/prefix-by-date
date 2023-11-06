mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

mod path_info;
pub use path_info::PathInfo;

use crate::context::Context;
use crate::reporter::Reporter;

use std::path::PathBuf;

pub struct Processing<'a> {
    context: &'a Context,
}

impl<'a> Processing<'a> {
    pub fn new(context: &'a Context) -> Processing<'a> {
        Self { context }
    }

    pub fn run(&self, paths: &Vec<PathBuf>) -> Result<()> {
        self.context.count(paths.len());

        for path in paths {
            self.context.processing(path);

            let path_info = PathInfo {
                context: self.context,
                path,
            };

            match path_info.prefix_if_possible() {
                Ok(replacement) => {
                    self.context
                        .processing_ok(path, replacement.result().as_str());
                }
                Err(error) => {
                    self.context.processing_err(path, &error);
                }
            }
        }

        Ok(())
    }
}
