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

    /// Transition current to Path
    ///
    /// Only possible from None (default state at the beginning) and Resolved
    pub fn set_current_path(&mut self, path: PathBuf) {
        if matches!(self.current, Current::None | Current::Resolved) {
            self.current = Current::Path(path);
            self.actions = Action::determine_for(&self.current);
        }
    }

    /// Transition current from Path or Resolving to Confirm
    ///
    /// The confirmation that transition to Resolving might not necessarily
    /// resolve the issue (e.g. Confirmation::Refuse)
    pub fn set_current_confirm(
        &mut self,
        replacement: Replacement,
        matchers: &[Box<dyn Matcher>],
    ) {
        if !matches!(self.current, Current::Path(_) | Current::Resolving(_, _))
        {
            return;
        }

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

    /// Transition current from Path to Rescue
    pub fn set_current_rescue(&mut self, replacement: Replacement) {
        if !matches!(self.current, Current::Path(_)) {
            return;
        }

        let change = Change::new(replacement);
        self.current = Current::Rescue(change);
        self.actions = Action::determine_for(&self.current);
    }

    /// Transition current from Confirm or Rescue to Resolving using the given
    /// confirmation, if that is allowed by the actions
    ///
    /// Returns true on transition, false otherwise
    pub fn set_current_resolving(&mut self, conf: Confirmation) -> bool {
        if matches!(self.current, Current::Confirm(_) | Current::Rescue(_))
            && self.actions.contains(&Action::from(&conf))
        {
            self.current.resolving(conf.clone());
            self.actions = Action::determine_for(&self.current);

            return true;
        }

        false
    }

    /// Transition from Resolving to Resolved, incrementing the progress
    /// tracker and logging the successful result
    pub fn set_current_success(&mut self, replacement: Replacement) {
        self.index += 1;
        self.logs.push(ProcessingResult::Success(replacement));
        self.current = Current::Resolved;
        self.actions = Action::determine_for(&self.current);
    }
    /// Transition from Resolving to Resolved, incrementing the progress
    /// tracker and logging the failed result
    pub fn set_current_failure(&mut self, path: PathBuf, error: String) {
        self.index += 1;
        self.logs.push(ProcessingResult::Failure(path, error));
        self.current = Current::Resolved;
        self.actions = Action::determine_for(&self.current);
    }

    /// Update the customize field of the current change, as returned by
    /// change()
    ///
    /// This also refresh the actions
    pub fn customize(&mut self, string: String) {
        if let Some(change) = self.change_mut() {
            change.customize = Some(string);
        }
        self.actions = Action::determine_for(&self.current);
    }

    /// Cancel current customization, i.e. sets the customize field of the
    /// current change back to None
    ///
    /// This also refresh the actions
    pub fn cancel_customize(&mut self) {
        if let Some(change) = self.change_mut() {
            change.customize = None;
        }
        self.actions = Action::determine_for(&self.current);
    }

    /// Get a Replacement from the customize field of the current change
    ///
    /// Returns None if there is no customization or if change() returns None
    pub fn customized_replacement(&self) -> Option<Replacement> {
        self.change().and_then(|change| {
            change.customize.clone().map(|value| {
                let mut replacement = change.replacement.clone();
                replacement.new_file_stem = value;
                replacement
            })
        })
    }

    /// Access the current change being considered for a Confirm or a Rescue
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

    /// Index of the current path being processed
    pub fn index(&self) -> usize {
        self.index
    }

    /// Number of paths to process
    pub fn len(&self) -> usize {
        self.len
    }

    /// State of the currently being processed path
    pub fn current(&self) -> &Current {
        &self.current
    }

    /// Actions that can be done at this time
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// List of the processing results
    pub fn logs(&self) -> &[ProcessingResult] {
        &self.logs
    }
}

/// Element currently being processed
#[derive(Debug, Clone, Default)]
pub enum Current {
    /// Default state on initialization
    #[default]
    None,
    /// The path is going to be processed
    ///
    /// From None and Resolved
    Path(PathBuf),
    /// There's a match for the path that we need to confirm
    ///
    /// From Path and Resolving
    Confirm(Change),
    /// There were no match for the path, but we can rescue from there
    ///
    /// From Path
    Rescue(Change),
    /// A decision has been taken (Confirmation).
    ///
    /// From Confirm and Rescue
    Resolving(Change, Confirmation),
    /// The path has been processed
    ///
    /// From Resolving
    Resolved,
}

impl PartialEq for Current {
    fn eq(&self, other: &Current) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Current {
    /// Change self to Resolving from a Confirm or Rescue by re-using the
    /// same Change
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
