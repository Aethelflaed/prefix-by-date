use crate::processing::Error;
use std::path::Path;

mod log_reporter;
pub use log_reporter::Log;

pub trait Reporter {
    /// Report the total count of elements to be processed
    fn count(&self, number: usize);

    /// Report that we start processing a new path
    fn processing(&self, path: &Path);
    /// Report that processing the path yielded an error
    fn processing_err(&self, path: &Path, error: &Error);
    /// Report that processing  the path finished successfully
    fn processing_ok(&self, path: &Path, new_path: &Path);
}