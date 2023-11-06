use std::path::{Path, PathBuf};
use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    NotFound(PathBuf),
    NoMatch(PathBuf),
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            Self::Io(error) => fmt::Display::fmt(&error, f),
            Self::NotFound(path) => {
                write!(f, "Path not found: {:?}", path)
            }
            Self::NoMatch(path) => {
                write!(f, "No match for path: {:?}", path)
            }
        }
    }
}

impl Error {
    pub fn not_found(path: &Path) -> Error {
        Self::NotFound(path.to_path_buf())
    }

    pub fn no_match(path: &Path) -> Error {
        Self::NoMatch(path.to_path_buf())
    }
}
