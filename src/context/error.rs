use crate::processing::Error as ProcessingError;

use std::{error, fmt, io};

use log::SetLoggerError as LogError;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    SetLoggerError(LogError),
    Processing(ProcessingError),
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<LogError> for Error {
    fn from(error: LogError) -> Self {
        Self::SetLoggerError(error)
    }
}

impl From<ProcessingError> for Error {
    fn from(error: ProcessingError) -> Self {
        Self::Processing(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::Io(error) => fmt::Display::fmt(&error, f),
            Self::SetLoggerError(error) => fmt::Display::fmt(&error, f),
            Self::Processing(error) => fmt::Display::fmt(&error, f),
        }
    }
}
