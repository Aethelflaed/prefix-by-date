use indicatif::ProgressBar;
use crate::reporter;
use crate::processing::Error;
use std::path::Path;

#[derive(Clone)]
pub struct Reporter {
    pub bar: ProgressBar,
}

impl Default for Reporter {
    fn default() -> Self {
        Self {
            bar: ProgressBar::new(1000)
        }
    }
}

impl reporter::Reporter for Reporter {
    /// Report the total count of elements to be processed
    fn count(&self, number: usize) {
        self.bar.set_length(number as u64);
    }

    /// Report that we start processing a new path
    fn processing(&self, _path: &Path) {
    }
    /// Report that processing the path yielded an error
    fn processing_err(&self, _path: &Path, _error: &Error) {
        self.bar.inc(1);
    }
    /// Report that processing  the path finished successfully
    fn processing_ok(&self, _path: &Path, _new_name: &str) {
        self.bar.inc(1);
    }
}
