use crate::reporter::Reporter;
use std::io::Error;
use std::path::Path;

#[derive(Default)]
pub struct Aggregate {
    reporters: Vec<Box<dyn Reporter>>,
}

impl Aggregate {
    pub fn add(&mut self, reporter: Box<dyn Reporter>) {
        self.reporters.push(reporter);
    }
}

impl Reporter for Aggregate {
    fn count(&self, number: usize) {
        for reporter in &self.reporters {
            reporter.count(number);
        }
    }

    fn processing(&self, path: &Path) {
        for reporter in &self.reporters {
            reporter.processing(path);
        }
    }
    fn processing_err(&self, path: &Path, error: &Error) {
        for reporter in &self.reporters {
            reporter.processing_err(path, error);
        }
    }
    fn processing_ok(&self, path: &Path) {
        for reporter in &self.reporters {
            reporter.processing_ok(path);
        }
    }
}
