use crate::processing::{Error, Result};

use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Replacement {
    pub path: PathBuf,
    pub new_file_stem: String,
    pub extension: String,
}

impl TryFrom<&Path> for Replacement {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self> {
        let file_stem: String = path
            .file_stem()
            .ok_or(Error::PathUnwrap(path.into(), "file_stem"))?
            .to_str()
            .ok_or(Error::PathUnwrap(path.into(), "file_stem/to_str"))?
            .into();
        let ext: String = path
            .extension()
            .ok_or(Error::PathUnwrap(path.into(), "extension"))?
            .to_str()
            .ok_or(Error::PathUnwrap(path.into(), "extension/to_str"))?
            .into();

        Ok(Replacement {
            path: PathBuf::from(path),
            new_file_stem: file_stem,
            extension: ext,
        })
    }
}

impl Replacement {
    pub fn execute(&self) -> Result<Self> {
        let new_path = self.new_path()?;
        std::fs::rename(&self.path, new_path)?;

        Ok(self.clone())
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
    fn try_from() {
        let replacement = Replacement::try_from(path().as_path()).unwrap();

        assert_eq!(String::from("test"), replacement.str_file_stem().unwrap());
        assert_eq!(path(), replacement.new_path().unwrap());
    }

    #[test]
    fn customized_file_stem() {
        let mut replacement = Replacement::try_from(path().as_path()).unwrap();
        replacement.new_file_stem = String::from("success.txt");

        assert_eq!(String::from("test"), replacement.str_file_stem().unwrap());
        assert_eq!(
            PathBuf::from("/this/is/a/success.txt.pdf"),
            replacement.new_path().unwrap()
        );
    }
}

impl fmt::Display for Replacement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}/{{{} => {}}}.{}",
            self.path.parent().unwrap().to_str().unwrap(),
            self.path.file_stem().unwrap().to_str().unwrap(),
            self.new_file_stem,
            self.extension
        )
    }
}
