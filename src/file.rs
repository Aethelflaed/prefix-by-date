use crate::state::State;
use chrono::{DateTime, Local};
use std::path::PathBuf;

fn format_date(date_time: &DateTime<Local>, state: &State) -> String {
    format!("{}", date_time.format(state.format.as_str()))
}

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

            let mut name = format_date(&replacement.date_time, state);

            if !replacement.rest.is_empty() {
                name.push_str(matcher.delimiter());
                name.push_str(replacement.rest.as_str());
            }

            rename(path, name.as_str())?;
        }
    }
    Ok(())
}
