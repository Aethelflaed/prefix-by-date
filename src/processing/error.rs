use std::{error, fmt, io};

pub enum ErrorKind {
    Io(io::Error),
    NotFound(String),
    NoMatch(String),
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
        write!(f, "An Error Occurred, Please Try Again!")
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An Error Occurred, Please Try Again!")
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Error
    {
        Error { kind }
    }
}
