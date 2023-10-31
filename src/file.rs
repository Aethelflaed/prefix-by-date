use crate::state::State;
use std::path::PathBuf;

fn rename(path: &PathBuf, new_name: &str) -> std::io::Result<()> {
    let mut new_path = path.clone();
    new_path.pop();
    new_path.push(new_name);

    log::info!("Renaming: {:?} -> {:?}", path, new_path);

    std::fs::rename(path, new_path)
}

pub fn prefix_file_if_possible(
    path: &PathBuf,
    state: &State,
) -> std::io::Result<()> {
    log::info!("Checking file: {:?}", path);

    let file_name = path.file_name().unwrap().to_str().unwrap();

    for matcher in &state.matchers {
        if let Some(replacement) = matcher.check(file_name) {
            log::debug!("Match: {}", matcher.name());

            rename(path, replacement.result(state).as_str())?;
        }
    }
    Ok(())
}
