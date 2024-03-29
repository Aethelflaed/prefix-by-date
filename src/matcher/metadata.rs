use crate::matcher::Matcher;
use crate::replacement::Replacement;

use chrono::{DateTime, Local};

#[derive(Default, Clone, Copy)]
enum Kind {
    #[default]
    Created,
    Modified,
}

impl Kind {
    fn name(&self) -> &'static str {
        match self {
            Kind::Created => CREATED,
            Kind::Modified => MODIFIED,
        }
    }
}

pub const CREATED: &str = "created";
pub const MODIFIED: &str = "modified";

#[derive(Default, Clone)]
pub struct Metadata {
    kind: Kind,
    format: String,
}

impl Metadata {
    pub fn new_created(format: &str) -> Self {
        Self::new(Kind::Created, format)
    }

    pub fn new_modified(format: &str) -> Self {
        Self::new(Kind::Modified, format)
    }

    fn new(kind: Kind, format: &str) -> Self {
        Self {
            kind,
            format: format.to_string(),
        }
    }
}

impl Matcher for Metadata {
    fn determine(
        &self,
        replacement: &Replacement,
    ) -> Option<(String, DateTime<Local>)> {
        let metadata = replacement.path().metadata().ok()?;
        let date_time = match self.kind {
            Kind::Created => metadata.created().ok()?,
            Kind::Modified => metadata.modified().ok()?,
        };

        Some((replacement.file_stem.clone(), date_time.into()))
    }

    /// Name of the matcher
    fn name(&self) -> &str {
        self.kind.name()
    }
    /// Delimiter to place between the matched elements
    fn delimiter(&self) -> &str {
        " "
    }
    /// Format to use for the date
    fn date_format(&self) -> &str {
        self.format.as_str()
    }

    fn auto_accept(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{assert_eq, assert_ne, test};
    use assert_fs::fixture::{FileTouch, NamedTempFile};

    use std::path::Path;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn check() {
        use crate::application::DEFAULT_DATE_TIME_FORMAT;

        let created = Metadata::new_created(DEFAULT_DATE_TIME_FORMAT);
        let modified = Metadata::new_modified(DEFAULT_DATE_TIME_FORMAT);

        let temp_file = NamedTempFile::new("foo").unwrap();
        let path = temp_file.path();
        std::fs::File::create(path).unwrap();

        assert!(created.check(path).is_some());

        let result_1 = modified.check(path);
        assert!(result_1.is_some());
        sleep(Duration::from_secs(1));

        temp_file.touch().unwrap();
        let result_2 = modified.check(path);
        assert!(result_2.is_some());

        assert_ne!(
            result_1.unwrap().new_file_stem,
            result_2.unwrap().new_file_stem
        );
    }

    #[test]
    fn check_unexisting_path() {
        assert!(Metadata::new_created("foo")
            .check(Path::new("foo"))
            .is_none());
    }

    #[test]
    fn name() {
        assert_eq!("created", Metadata::new_created("foo").name());
        assert_eq!("modified", Metadata::new_modified("foo").name());
    }

    #[test]
    fn delimiter() {
        assert_eq!(" ", Metadata::new_created("foo").delimiter());
    }

    #[test]
    fn date_format() {
        use crate::application::DEFAULT_DATE_FORMAT;

        assert_eq!("foo", Metadata::new_created("foo").date_format());
        assert_eq!(
            DEFAULT_DATE_FORMAT,
            Metadata::new_created(DEFAULT_DATE_FORMAT).date_format()
        );
    }

    #[test]
    fn auto_accept() {
        assert!(!Metadata::new_created("foo").auto_accept());
    }
}
