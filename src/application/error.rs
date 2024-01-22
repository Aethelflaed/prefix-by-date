use crate::processing::Error as ProcessingError;

use std::{error, fmt, io};

use log::SetLoggerError as LogError;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    SetLogger(LogError),
    Processing(ProcessingError),
    Custom(String),
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<LogError> for Error {
    fn from(error: LogError) -> Self {
        Self::SetLogger(error)
    }
}

impl From<ProcessingError> for Error {
    fn from(error: ProcessingError) -> Self {
        Self::Processing(error)
    }
}

impl From<&'static str> for Error {
    fn from(error: &'static str) -> Self {
        Self::Custom(error.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Self::Custom(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::Io(error) => fmt::Display::fmt(&error, f),
            Self::SetLogger(error) => fmt::Display::fmt(&error, f),
            Self::Processing(error) => fmt::Display::fmt(&error, f),
            Self::Custom(error) => fmt::Display::fmt(&error, f),
        }
    }
}
