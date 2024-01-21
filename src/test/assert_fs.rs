pub use assert_fs::{TempDir, fixture::{FixtureError, ChildPath}, prelude::*};

use std::path;

pub trait PathExistingChild {
    fn existing_child<P>(&self, path: P) -> Result<ChildPath, FixtureError>
    where
        P: AsRef<path::Path>;
}

impl PathExistingChild for TempDir {
    fn existing_child<P>(&self, path: P) -> Result<ChildPath, FixtureError>
    where
        P: AsRef<path::Path>
    {
        let child = self.child(path);
        child.touch()?;
        Ok(child)
    }
}

impl PathExistingChild for ChildPath {
    fn existing_child<P>(&self, path: P) -> Result<ChildPath, FixtureError>
    where
        P: AsRef<path::Path>
    {
        let child = self.child(path);
        child.touch()?;
        Ok(child)
    }
}
