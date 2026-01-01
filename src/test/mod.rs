pub use test_log::test;

pub use pretty_assertions::{assert_eq, assert_ne};

pub mod assert_fs;
pub mod matchers;
pub mod paths;

pub fn with_temp_dir<F, R>(function: F) -> R
where
    F: FnOnce(&assert_fs::TempDir) -> R,
{
    let temp = assert_fs::TempDir::new().unwrap();
    let result = function(&temp);

    // The descrutor would silence any issue, so we call close() explicitly
    temp.close().unwrap();

    result
}

pub fn with_config_dir<F, R>(function: F) -> R
where
    F: FnOnce(&assert_fs::TempDir) -> R,
{
    with_temp_dir(|temp| {
        temp_env::with_var(
            "PREFIX_BY_DATE_CONFIG",
            Some(temp.path().as_os_str()),
            || function(temp),
        )
    })
}

pub fn with_config_copied<T, R, S>(patterns: &[S], function: T) -> R
where
    T: FnOnce() -> R,
    S: AsRef<str>,
{
    use ::assert_fs::fixture::PathCopy;

    with_config_dir(|temp| {
        temp.copy_from(paths::FIXTURES.as_path(), patterns).unwrap();

        function()
    })
}

pub fn with_config<T, R>(function: T) -> R
where
    T: FnOnce() -> R,
{
    with_config_copied(&["config.toml"], function)
}
