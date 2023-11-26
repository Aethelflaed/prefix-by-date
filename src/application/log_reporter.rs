use crate::processing::Error;
use crate::replacement::Replacement;

use std::cell::Cell;
use std::path::Path;

#[derive(Default)]
pub struct LogReporter {
    count: Cell<usize>,
    current: Cell<usize>,
}

impl LogReporter {
    /// Report the total count of elements to be processed
    pub fn count(&self, number: usize) {
        self.count.set(number);
        log::info!("Processing {} paths...", number);
    }

    /// Report that we start processing a new path
    pub fn processing(&self, path: &Path) {
        self.current.set(self.current.get() + 1);
        self.report_path("Processing path", path);
    }

    /// Report that processing the path yielded an error
    pub fn processing_err(&self, path: &Path, error: &Error) {
        self.report_path("Error processing path", path);
        log::error!("{}", error);
    }

    /// Report that processing  the path finished successfully
    pub fn processing_ok(&self, replacement: &Replacement) {
        self.report_path("Success processing path", &replacement.path);
        log::info!("Into: {}", replacement);
    }

    fn report_path(&self, message: &str, path: &Path) {
        log::info!(
            "{} {}/{}: {:?}",
            message,
            self.current.get(),
            self.count.get(),
            path
        );
    }
}
