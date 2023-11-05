use std::path::PathBuf;
use std::{error, fmt, io};

pub enum ErrorKind {
    Io(io::Error),
    NotFound(PathBuf),
    NoMatch(PathBuf),
}

pub struct Error {
    kind: ErrorKind,
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error {
            kind: ErrorKind::Io(error),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::Io(error) => write!(f, "Io error: {:?}", error),
            ErrorKind::NotFound(path) => {
                write!(f, "Path not found: {:?}", path)
            }
            ErrorKind::NoMatch(path) => {
                write!(f, "No match for path: {:?}", path)
            }
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::Io(error) => write!(f, "Io error: {:?}", error),
            ErrorKind::NotFound(path) => {
                write!(f, "Path not found: {:?}", path)
            }
            ErrorKind::NoMatch(path) => {
                write!(f, "No match for path: {:?}", path)
            }
        }
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Error {
        Error { kind }
    }
}
