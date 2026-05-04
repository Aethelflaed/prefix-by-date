use std::path::{Path, PathBuf};
use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    NotFound(PathBuf),
    NoMatch(PathBuf),
    PathUnwrap(PathBuf, &'static str),
    Skip(PathBuf),
    Abort,
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
                write!(f, "Path not found: {}", path.display())
            }
            Self::NoMatch(path) => {
                write!(f, "No match for path: {}", path.display())
            }
            Self::PathUnwrap(path, op) => {
                write!(
                    f,
                    "Error unwrapping operationg {op:?} for path: {}",
                    path.display()
                )
            }
            Self::Skip(path) => {
                write!(f, "Skipping {}", path.display())
            }
            Self::Abort => {
                write!(f, "Abort received, quitting...")
            }
        }
    }
}

impl Error {
    pub fn not_found(path: &Path) -> Self {
        Self::NotFound(path.to_path_buf())
    }

    pub fn no_match(path: &Path) -> Self {
        Self::NoMatch(path.to_path_buf())
    }
}
