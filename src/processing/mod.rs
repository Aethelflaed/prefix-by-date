mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

mod path_info;
pub use path_info::PathInfo;

use crate::application::Application;
use crate::reporter::Reporter;

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

            let path_info = PathInfo {
                app: self.app,
                path,
            };

            match path_info.prefix_if_possible() {
                Ok(replacement) => {
                    self.app
                        .processing_ok(path, replacement.result().as_str());
                }
                Err(error) => {
                    self.app.processing_err(path, &error);
                }
            }
        }

        Ok(())
    }
}
