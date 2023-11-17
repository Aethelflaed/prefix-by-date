use crate::processing::{Error, Result};

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub struct Replacement {
    pub path: PathBuf,
    pub new_file_stem: String,
    pub extension: String,
}

impl Replacement {
    pub fn from(path: &Path) -> Option<Self> {
        let file_stem = path.file_stem().and_then(os_str_to_string)?;
        let ext = path.extension().and_then(os_str_to_string)?;

        Some(Replacement {
            path: PathBuf::from(path),
            new_file_stem: file_stem,
            extension: ext,
        })
    }

    pub fn execute(&self) -> Result<PathBuf> {
        let new_path = self.new_path()?;
        std::fs::rename(&self.path, &new_path)?;

        Ok(new_path)
    }

    pub fn str_file_stem(&self) -> Option<String> {
        self.path.file_stem().and_then(os_str_to_string)
    }

    pub fn new_path(&self) -> Result<PathBuf> {
        let parent = self
            .path
            .parent()
            .ok_or(Error::PathUnwrap(self.path.clone(), "parent"))?;
        let extension = self
            .path
            .extension()
            .ok_or(Error::PathUnwrap(self.path.clone(), "extension"))?
            .to_str()
            .ok_or(Error::PathUnwrap(self.path.clone(), "extension/to_str"))?;

        Ok(parent.join(format!("{}.{}", self.new_file_stem, extension,)))
    }
}

fn os_str_to_string(os_str: &OsStr) -> Option<String> {
    os_str.to_str().map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn path() -> PathBuf {
        PathBuf::from("/this/is/a/test.pdf")
    }

    #[test]
    fn from() {
        let replacement = Replacement::from(&path()).unwrap();

        assert_eq!(String::from("test"), replacement.str_file_stem().unwrap());
        assert_eq!(path(), replacement.new_path().unwrap());
    }

    #[test]
    fn customized_file_stem() {
        let mut replacement = Replacement::from(&path()).unwrap();
        replacement.new_file_stem = String::from("success.txt");

        assert_eq!(String::from("test"), replacement.str_file_stem().unwrap());
        assert_eq!(
            PathBuf::from("/this/is/a/success.txt.pdf"),
            replacement.new_path().unwrap()
        );
    }
}
