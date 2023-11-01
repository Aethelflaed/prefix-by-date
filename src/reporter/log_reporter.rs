use crate::reporter::Reporter;
use std::cell::Cell;
use std::io::Error;
use std::path::Path;

#[derive(Default)]
pub struct Log {
    count: Cell<usize>,
    current: Cell<usize>,
}

impl Log {
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

impl Reporter for Log {
    /// Report the total count of elements to be processed
    fn count(&self, number: usize) {
        self.count.set(number);
        log::info!("Processing {} paths...", number);
    }

    /// Report that we start processing a new path
    fn processing(&self, path: &Path) {
        self.current.set(self.current.get() + 1);
        self.report_path("Processing path", path);
    }
    /// Report that processing the path yielded an error
    fn processing_err(&self, path: &Path, error: &Error) {
        self.report_path("Error processing path", path);
        log::error!("{:?}", error);
    }
    /// Report that processing  the path finished successfully
    fn processing_ok(&self, path: &Path) {
        self.report_path("Success processing path", path);
    }
}
