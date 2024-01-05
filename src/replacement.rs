use crate::processing::{Error, Result};

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct Replacement {
    pub parent: PathBuf,
    pub file_stem: String,
    pub new_file_stem: String,
    pub extension: String,
}

impl TryFrom<&Path> for Replacement {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self> {
        let parent = path
            .parent()
            .ok_or(Error::PathUnwrap(path.into(), "parent"))?;
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

        // Try to resolve the path, but rescue silently if it doesn't work
        let parent = parent.canonicalize().unwrap_or(parent.to_path_buf());

        Ok(Replacement {
            parent,
            file_stem: file_stem.clone(),
            new_file_stem: file_stem,
            extension: ext,
        })
    }
}

impl Replacement {
    pub fn execute(&self) -> Result<Self> {
        std::fs::rename(self.path(), self.new_path())?;

        Ok(self.clone())
    }

    pub fn file_name(&self) -> String {
        format!("{}.{}", self.file_stem, self.extension)
    }

    pub fn new_file_name(&self) -> String {
        format!("{}.{}", self.new_file_stem, self.extension)
    }

    pub fn path(&self) -> PathBuf {
        self.parent.join(self.file_name())
    }

    pub fn new_path(&self) -> PathBuf {
        self.parent.join(self.new_file_name())
    }

    pub fn new_file_stem(mut self, value: String) -> Self {
        self.new_file_stem = value;
        self
    }
}

impl fmt::Display for Replacement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}/{{{} => {}}}.{}",
            self.parent.to_str().unwrap(),
            self.file_stem,
            self.new_file_stem,
            self.extension
        )
    }
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

        assert_eq!(String::from("test"), replacement.file_stem);
        assert_eq!(path(), replacement.new_path());
    }

    #[test]
    fn customized_file_stem() {
        let mut replacement = Replacement::try_from(path().as_path()).unwrap();
        replacement.new_file_stem = String::from("success.txt");

        assert_eq!(String::from("test"), replacement.file_stem);
        assert_eq!(
            PathBuf::from("/this/is/a/success.txt.pdf"),
            replacement.new_path()
        );
    }

    #[test]
    fn new_file_stem_fn() {
        let replacement = Replacement::try_from(path().as_path()).unwrap().new_file_stem("success".to_string());
        assert_eq!(
            PathBuf::from("/this/is/a/success.pdf"),
            replacement.new_path()
        );
    }
}
