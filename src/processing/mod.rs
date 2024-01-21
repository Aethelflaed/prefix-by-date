use crate::matcher::Matcher;
use crate::replacement::Replacement;

mod error;
pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;

mod matcher;
pub use matcher::ProcessingMatcher;

mod log_reporter;
mod notif_reporter;

use std::boxed::Box;
use std::path::{Path, PathBuf};

pub struct Processing<'a, T>
where
    T: Communication,
{
    matchers: Vec<ProcessingMatcher<'a>>,
    paths: &'a [PathBuf],
    interface: &'a T,
    reporters: Vec<Box<dyn Reporter>>,
}

pub trait Reporter {
    /// Report the total count of elements about to be processed
    fn setup(&self, count: usize);
    /// Start processing this path
    fn processing(&self, path: &Path);
    /// Processing went well and ended-up with this replacement
    fn processing_ok(&self, replacement: &Replacement);
    /// Processing encountered this error
    fn processing_err(&self, path: &Path, error: &Error);
}

pub trait Communication: Reporter {
    /// Whenever a matcher finds a replacement, confirm it
    fn confirm(&self, replacement: &Replacement) -> Confirmation;
    /// If no match is found, attempt to rescue the Error::NoMatch
    fn rescue(&self, error: Error) -> Result<Replacement>;
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Confirmation {
    Accept,
    Always,
    Skip,
    Refuse,
    Ignore,
    Abort,
    Replace(Replacement),
}

impl PartialEq for Confirmation {
    fn eq(&self, other: &Confirmation) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl<'a, T> Processing<'a, T>
where
    T: Communication,
{
    pub fn new(
        interface: &'a T,
        matchers: &'a [Box<dyn Matcher>],
        paths: &'a [PathBuf],
    ) -> Self {
        Self {
            matchers: matchers.iter().map(From::<_>::from).collect(),
            paths,
            interface,
            reporters: vec![
                Box::<log_reporter::LogReporter>::default(),
                #[cfg(feature = "notif")]
                Box::<notif_reporter::NotifReporter>::default(),
            ],
        }
    }

    pub fn run(&mut self) -> Result<()> {
        if self.paths.is_empty() || self.matchers.is_empty() {
            return Ok(());
        }

        self.report_setup(self.paths.len());

        for path in self.paths {
            self.report_processing(path);

            match self
                .prefix_if_possible(path)
                .and_then(|replacement| replacement.execute())
            {
                Ok(replacement) => {
                    self.report_processing_ok(&replacement);
                }
                Err(error) => {
                    self.report_processing_err(path, &error);

                    if let Error::Abort = error {
                        return Err(error);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn prefix_if_possible(&mut self, path: &Path) -> Result<Replacement> {
        if !path.try_exists().unwrap() {
            return Err(Error::not_found(path));
        }

        // Get an immutable ref
        let interface: &T = self.interface;

        let mut found = false;

        for matcher in self
            .matchers
            .iter_mut()
            .filter(|matcher| !matcher.ignored())
        {
            if let Some(replacement) = matcher.check(path) {
                found = true;
                if matcher.confirmed() {
                    return Ok(replacement);
                }
                match interface.confirm(&replacement) {
                    Confirmation::Accept => return Ok(replacement),
                    Confirmation::Always => {
                        matcher.confirm();
                        return Ok(replacement);
                    }
                    Confirmation::Skip => {
                        return Err(Error::Skip(path.to_path_buf()));
                    }
                    Confirmation::Refuse => {}
                    Confirmation::Ignore => {
                        matcher.ignore();
                    }
                    Confirmation::Abort => {
                        return Err(Error::Abort);
                    }
                    Confirmation::Replace(replacement) => {
                        return Ok(replacement)
                    }
                };
            }
        }

        if found {
            Err(Error::no_match(path))
        } else {
            interface.rescue(Error::no_match(path))
        }
    }

    fn report_setup(&self, count: usize) {
        for reporter in &self.reporters {
            reporter.setup(count);
        }

        self.interface.setup(count);
    }
    fn report_processing(&self, path: &Path) {
        for reporter in &self.reporters {
            reporter.processing(path);
        }

        self.interface.processing(path);
    }
    fn report_processing_ok(&self, replacement: &Replacement) {
        for reporter in &self.reporters {
            reporter.processing_ok(replacement);
        }

        self.interface.processing_ok(replacement);
    }
    fn report_processing_err(&self, path: &Path, error: &Error) {
        for reporter in &self.reporters {
            reporter.processing_err(path, error);
        }

        self.interface.processing_err(path, error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::{matchers, test, with_temp_dir, assert_fs::*};
    use mockall::*;

    mock! {
        pub Interface {}
        impl Reporter for Interface {
            fn setup(&self, count: usize);
            fn processing(&self, path: &Path);
            fn processing_ok(&self, replacement: &Replacement);
            fn processing_err(&self, path: &Path, error: &Error);
        }
        impl Communication for Interface {
            fn confirm(&self, replacement: &Replacement) -> Confirmation;
            fn rescue(&self, error: Error) -> Result<Replacement>;
        }
    }

    // Ensure no work is done if we have either no matchers or no paths
    #[test]
    fn empty_paths_and_or_matchers() -> Result<()> {
        let mut interface = MockInterface::new();
        let matchers = [matchers::today_boxed()];
        let paths = [PathBuf::from("foo")];

        interface.expect_setup().never();

        let mut processing = Processing::new(&interface, &[], &[]);
        processing.run()?;

        let mut processing = Processing::new(&interface, &matchers, &[]);
        processing.run()?;

        let mut processing = Processing::new(&interface, &[], &paths);
        processing.run()?;

        Ok(())
    }

    // Ensure early return in case the path does not exist
    #[test]
    fn unexisting_path() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::today_boxed()];
            let path = temp.path().join("foo");
            let paths = [path.clone()];

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::NotFound(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface.expect_processing_ok().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()
        })
    }

    // Ensure rescue is called when there is no match, returning an error
    #[test]
    fn rescue_and_return_error() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::weird_boxed()];
            let child = temp.existing_child("foo").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone()];

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_rescue()
                .withf(|e| matches!(e, Error::NoMatch(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|e| Err(e));
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::NoMatch(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface.expect_processing_ok().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()
        })
    }

    // Ensure rescue is called when there is no match, returning a replacement
    #[test]
    fn rescue_and_return_replacement() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::weird_boxed()];
            let child = temp.existing_child("foo").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone()];

            let mut replacement = Replacement::try_from(child.path())?;
            replacement.new_file_stem = String::from("bar");

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_rescue()
                .withf(|e| matches!(e, Error::NoMatch(_)))
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Ok(replacement));
            interface
                .expect_processing_ok()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface.expect_processing_err().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()?;

            temp.child("foo").assert(predicate::path::missing());
            temp.child("bar").assert(predicate::path::exists());

            Ok(())
        })
    }

    // Uses rescue to provide a replacement that is buggy: the initial path()
    // it is constructed from does not exists so Replacement::execute fails
    #[test]
    fn rescue_and_return_erroneous_replacement() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::weird_boxed()];
            let child = temp.existing_child("foo").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone()];

            let replacement = Replacement::try_from(temp.child("bar").path())?;

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_rescue()
                .withf(|e| matches!(e, Error::NoMatch(_)))
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Ok(replacement));
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::Io(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface.expect_processing_ok().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()
        })
    }

    // Ensure accepted replacement is executed
    #[test]
    fn confirm_accept() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::ymd_boxed()];

            let child = temp.existing_child("foo 20240120").unwrap();
            let path = child.to_path_buf();

            let child2 = temp.existing_child("bar 20240120").unwrap();
            let path2 = child2.to_path_buf();

            let paths = [path.clone(), path2.clone()];

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .with(predicate::eq(2))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Confirmation::Accept);
            interface
                .expect_processing_ok()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path2))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Confirmation::Accept);
            interface
                .expect_processing_ok()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()?;

            log::error!(
                "temp dir: {:?}",
                temp.read_dir().unwrap().collect::<Vec<_>>()
            );

            child.assert(predicate::path::missing());
            temp.child("2024-01-20 foo")
                .assert(predicate::path::exists());

            child2.assert(predicate::path::missing());
            temp.child("2024-01-20 bar")
                .assert(predicate::path::exists());

            Ok(())
        })
    }

    // Ensure always accepts replacement and all successive replacement from
    // the same matcher
    #[test]
    fn confirm_always() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::ymd_boxed()];

            let child = temp.existing_child("foo 20240120").unwrap();
            let path = child.to_path_buf();

            let child2 = temp.existing_child("bar 20240120").unwrap();
            let path2 = child2.to_path_buf();

            let paths = [path.clone(), path2.clone()];

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .with(predicate::eq(2))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Confirmation::Always);
            interface
                .expect_processing_ok()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path2))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing_ok()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()?;

            log::error!(
                "temp dir: {:?}",
                temp.read_dir().unwrap().collect::<Vec<_>>()
            );

            child.assert(predicate::path::missing());
            temp.child("2024-01-20 foo")
                .assert(predicate::path::exists());

            child2.assert(predicate::path::missing());
            temp.child("2024-01-20 bar")
                .assert(predicate::path::exists());

            Ok(())
        })
    }

    // Ensure second matcher is not considered if the path is skipped
    #[test]
    fn confirm_skip() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::ymd_boxed(), matchers::ymd_boxed()];
            let child = temp.existing_child("foo 20240120").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone()];

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Confirmation::Skip);
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::Skip(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface.expect_processing_ok().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()
        })
    }

    // Ensure we try the the second matcher if the first is refused and that
    // rescue is not called anyway
    #[test]
    fn confirm_refuse_twice_and_return_no_match_without_rescue() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::ymd_boxed(), matchers::ymd_boxed()];
            let child = temp.existing_child("foo 20240120").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone()];

            let mut replacement = Replacement::try_from(child.path())?;
            replacement.new_file_stem = String::from("bar");

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(2)
                .in_sequence(&mut seq)
                .returning(|_| Confirmation::Refuse);
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::NoMatch(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface.expect_processing_ok().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()
        })
    }

    // Ensure second path has no match (and needs to be rescued) if matcher
    // is ignored on first path
    #[test]
    fn confirm_ignore() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::ymd_boxed()];
            let child = temp.existing_child("foo 20240120").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone(), path.clone()];

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Confirmation::Ignore);
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::NoMatch(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_rescue()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|e| Err(e));
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::NoMatch(_)))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface.expect_processing_ok().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()
        })
    }

    // Ensure early return with abort
    #[test]
    fn confirm_abort() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::ymd_boxed(), matchers::ymd_boxed()];
            let child = temp.existing_child("foo 20240120").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone(), path.clone()];

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| Confirmation::Abort);
            interface
                .expect_processing_err()
                .withf(|_, e| matches!(e, Error::Abort))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_, _| {});
            interface.expect_processing_ok().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            assert!(matches!(processing.run(), Err(Error::Abort)));

            Ok(())
        })
    }

    // Ensure replacement given is executed
    #[test]
    fn confirm_replace() -> Result<()> {
        with_temp_dir(|temp| {
            let mut interface = MockInterface::new();
            let matchers = [matchers::ymd_boxed()];
            let child = temp.existing_child("foo 20240120").unwrap();
            let path = child.to_path_buf();
            let paths = [path.clone()];

            let mut replacement = Replacement::try_from(child.path())?;
            replacement.new_file_stem = String::from("bar");

            let mut seq = Sequence::new();
            interface
                .expect_setup()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_processing()
                .with(predicate::eq(path.clone()))
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface
                .expect_confirm()
                .times(1)
                .in_sequence(&mut seq)
                .return_once(move |_| Confirmation::Replace(replacement));
            interface
                .expect_processing_ok()
                .times(1)
                .in_sequence(&mut seq)
                .returning(|_| {});
            interface.expect_processing_err().never();

            let mut processing = Processing::new(&interface, &matchers, &paths);
            processing.run()?;

            child.assert(predicate::path::missing());
            temp.child("bar").assert(predicate::path::exists());

            Ok(())
        })
    }
}
