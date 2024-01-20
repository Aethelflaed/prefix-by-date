use once_cell::sync::Lazy;
use std::path::PathBuf;

pub static TESTS: Lazy<PathBuf> = Lazy::new(build_tests_path);
pub static FIXTURES: Lazy<PathBuf> = Lazy::new(build_fixtures_path);

fn build_tests_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn build_fixtures_path() -> PathBuf {
    TESTS.join("fixtures")
}
