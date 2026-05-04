use std::path::PathBuf;

pub static TESTS: std::sync::LazyLock<PathBuf> =
    std::sync::LazyLock::new(build_tests_path);
pub static FIXTURES: std::sync::LazyLock<PathBuf> =
    std::sync::LazyLock::new(build_fixtures_path);

fn build_tests_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

fn build_fixtures_path() -> PathBuf {
    TESTS.join("fixtures")
}
