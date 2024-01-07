use crate::matcher::Matcher;
use crate::processing::Confirmation;
use crate::replacement::Replacement;
use crate::ui::actions::Action;

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
    actions: Vec<Action>,
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
        self.actions = Action::determine_for(&self.current);
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
        self.actions = Action::determine_for(&self.current);
    }
    pub fn set_current_rescue(&mut self, replacement: Replacement) {
        let change = Change::new(replacement);
        self.current = Current::Rescue(change);
        self.actions = Action::determine_for(&self.current);
    }
    pub fn set_current_resolving(&mut self, conf: Confirmation) {
        self.current.resolving(conf);
        self.actions = Action::determine_for(&self.current);
    }

    pub fn success(&mut self, replacement: Replacement) {
        self.index += 1;
        self.logs.push(ProcessingResult::Success(replacement));
        self.current = Current::Resolved;
        self.actions = Action::determine_for(&self.current);
    }
    pub fn failure(&mut self, path: PathBuf, error: String) {
        self.index += 1;
        self.logs.push(ProcessingResult::Failure(path, error));
        self.current = Current::Resolved;
        self.actions = Action::determine_for(&self.current);
    }

    pub fn customize(&mut self, string: String) {
        self.change_mut()
            .map(|change| change.customize = Some(string));
        self.actions = Action::determine_for(&self.current);
    }

    pub fn cancel_customize(&mut self) {
        self.change_mut().map(|change| change.customize = None);
        self.actions = Action::determine_for(&self.current);
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

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    pub fn logs(&self) -> &[ProcessingResult] {
        &self.logs
    }
}

/// Element currently being processed
#[derive(Debug, Clone, Default)]
pub enum Current {
    #[default]
    None,
    Path(PathBuf),
    Confirm(Change),
    Rescue(Change),
    Resolving(Change, Confirmation),
    Resolved,
}

impl PartialEq for Current {
    fn eq(&self, other: &Current) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Current {
    fn resolving(&mut self, conf: Confirmation) {
        use Current::*;

        match self {
            Confirm(_) | Rescue(_) => {
                let old = std::mem::take(self);
                if let Confirm(change) = old {
                    let _ = std::mem::replace(self, Resolving(change, conf));
                } else if let Rescue(change) = old {
                    let _ = std::mem::replace(self, Resolving(change, conf));
                }
            }
            _ => {}
        }
    }
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
    use pretty_assertions::assert_eq;

    fn path() -> PathBuf {
        PathBuf::from("/this/is/a/test")
    }

    fn change() -> Change {
        Change {
            replacement: Replacement::try_from(path().as_path()).unwrap(),
            alternatives: HashMap::from([(
                "Hello".to_string(),
                Replacement::default(),
            )]),
            customize: None,
        }
    }

    fn test_current_resolving(mut current: Current) {
        match current.clone() {
            Current::None => {
                let mut current = Current::None;
                current.resolving(Confirmation::Abort);
                assert_eq!(Current::None, current);
            }
            Current::Path(path) => {
                current.resolving(Confirmation::Abort);
                assert_eq!(Current::Path(path), current);
            }
            Current::Confirm(change) => {
                current.resolving(Confirmation::Abort);
                assert_eq!(
                    Current::Resolving(change, Confirmation::Abort),
                    current
                );
            }
            Current::Rescue(change) => {
                current.resolving(Confirmation::Abort);
                assert_eq!(
                    Current::Resolving(change, Confirmation::Abort),
                    current
                );
            }
            Current::Resolving(change, conf) => {
                current.resolving(Confirmation::Abort);
                assert_eq!(Current::Resolving(change, conf), current);
            }
            Current::Resolved => {
                current.resolving(Confirmation::Abort);
                assert_eq!(Current::Resolved, current);
            }
        }
    }

    #[test]
    fn current_resolving_none() {
        test_current_resolving(Current::None);
    }

    #[test]
    fn current_resolving_path() {
        test_current_resolving(Current::Path(path()));
    }

    #[test]
    fn current_resolving_confirm() {
        test_current_resolving(Current::Confirm(change()));
    }

    #[test]
    fn current_resolving_rescue() {
        test_current_resolving(Current::Rescue(change()));
    }

    #[test]
    fn current_resolving_resolving() {
        test_current_resolving(Current::Resolving(
            change(),
            Confirmation::Accept,
        ));
    }

    #[test]
    fn current_resolving_resolved() {
        test_current_resolving(Current::Resolved);
    }
}
