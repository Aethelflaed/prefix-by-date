use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::replacement::Replacement;
use crate::ui::actions::Actions;

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct State {
    /// Currently processing item index
    index: usize,
    /// Total number of items to process
    len: usize,
    /// What is currently being processed
    current: Current,
    /// Relevant actions for the current item
    actions: Actions,
    logs: Vec<ProcessingResult>,
}

impl State {
    pub fn new(len: usize) -> Self {
        Self {
            len,
            ..Default::default()
        }
    }

    pub fn set_current_path(&mut self, path: PathBuf) {
        self.current = Current::Path(path);
        self.actions = Actions::from(&self.current);
    }
    pub fn set_current_confirm(
        &mut self,
        replacement: Replacement,
        matchers: &[Box<dyn Matcher>],
    ) {
        let mut change = Change::new(replacement.clone());
        let path_buf = replacement.path();
        let path = path_buf.as_path();

        change.alternatives = matchers
            .iter()
            .filter_map(|matcher| {
                matcher.check(path).and_then(|rep| {
                    // Skip alternatives similar to the replacement
                    if rep.new_file_stem == replacement.new_file_stem {
                        None
                    } else {
                        Some((matcher.name().to_string(), rep))
                    }
                })
            })
            .collect();
        self.current = Current::Confirm(change);
        self.actions = Actions::from(&self.current);
    }
    pub fn set_current_rescue(&mut self, replacement: Replacement) {
        let change = Change::new(replacement);
        self.current = Current::Rescue(change);
        self.actions = Actions::from(&self.current);
    }
    pub fn set_current_resolving(&mut self, conf: Confirmation) {
        if let Some(change) = self.change() {
            self.current = Current::Resolving(change.clone(), conf);
        }
    }

    pub fn success(&mut self, replacement: Replacement) {
        self.index += 1;
        self.logs.push(ProcessingResult::Success(replacement));
        self.current = Current::Resolved;
    }
    pub fn failure(&mut self, path: PathBuf, error: String) {
        self.index += 1;
        self.logs.push(ProcessingResult::Failure(path, error));
        self.current = Current::Resolved;
    }

    pub fn customize(&mut self, string: String) {
        self.change_mut()
            .map(|change| change.customize = Some(string));
        self.actions = Actions::from(&self.current);
    }

    pub fn cancel_customize(&mut self) {
        self.change_mut().map(|change| change.customize = None);
        self.actions = Actions::from(&self.current);
    }

    pub fn change(&self) -> Option<&Change> {
        match &self.current {
            Current::Confirm(change) | Current::Rescue(change) => Some(change),
            _ => None,
        }
    }
    fn change_mut(&mut self) -> Option<&mut Change> {
        match &mut self.current {
            Current::Confirm(change) | Current::Rescue(change) => Some(change),
            _ => None,
        }
    }

    pub fn customized_replacement(&self) -> Option<Replacement> {
        self.change().and_then(|change| {
            if let Some(value) = change.customize.clone() {
                Some(change.replacement.clone().new_file_stem(value))
            } else {
                None
            }
        })
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn current(&self) -> &Current {
        &self.current
    }

    pub fn actions(&self) -> &Actions {
        &self.actions
    }

    pub fn logs(&self) -> &[ProcessingResult] {
        &self.logs
    }
}

/// Element currently being processed
#[derive(Debug, Default)]
pub enum Current {
    #[default]
    None,
    Path(PathBuf),
    Confirm(Change),
    Rescue(Change),
    Resolving(Change, Confirmation),
    Resolved,
}

#[derive(Debug, Clone, Default)]
pub struct Change {
    pub replacement: Replacement,
    pub alternatives: HashMap<String, Replacement>,
    pub customize: Option<String>,
}

impl Change {
    pub fn new(replacement: Replacement) -> Self {
        Self {
            replacement,
            ..Default::default()
        }
    }

    /// Check if we can start customizing the change
    ///
    /// The change can be customized if it hasn't been customized yet (in which
    /// case, you can just edit it), or if there are alternatives.
    pub fn is_further_customizable(&self) -> bool {
        self.customize.is_none() || !self.alternatives.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum ProcessingResult {
    Success(Replacement),
    Failure(PathBuf, String),
}

impl std::fmt::Display for ProcessingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Success(rep) => write!(f, "{}", rep),
            Self::Failure(_path, error) => write!(f, "{}", error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
