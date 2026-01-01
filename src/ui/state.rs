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
        if !matches!(self.current, Current::Path(_) | Current::Resolving(_)) {
            return;
        }

        let path_buf = replacement.path();
        let path = path_buf.as_path();

        let alternatives = matchers
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

        let mut change = Change::new(replacement);
        change.alternatives = alternatives;

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
            self.current = Current::Resolving(conf);
            self.actions = Action::determine_for(&self.current);

            return true;
        }

        false
    }

    /// Transition from Resolving to Resolved, incrementing the progress
    /// tracker and logging the successful result
    pub fn set_current_success(&mut self, replacement: Replacement) {
        if !matches!(self.current, Current::Resolving(_)) {
            return;
        }

        self.index += 1;
        self.logs.push(ProcessingResult::Success(replacement));
        self.current = Current::Resolved;
        self.actions = Action::determine_for(&self.current);
    }
    /// Transition from Resolving to Resolved, incrementing the progress
    /// tracker and logging the failed result
    pub fn set_current_failure(&mut self, path: PathBuf, error: String) {
        if !matches!(self.current, Current::Resolving(_)) {
            return;
        }

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

            self.actions = Action::determine_for(&self.current);
        }
    }

    /// Cancel current customization, i.e. sets the customize field of the
    /// current change back to None
    ///
    /// This also refresh the actions
    pub fn cancel_customize(&mut self) {
        if let Some(change) = self.change_mut() {
            change.customize = None;

            self.actions = Action::determine_for(&self.current);
        }
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
    Resolving(Confirmation),
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

#[derive(Debug, Clone, PartialEq)]
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
    use crate::test::{assert_eq, matchers, test};

    #[derive(Default)]
    struct CurrentIterator {
        state: Current,
    }

    impl Iterator for CurrentIterator {
        type Item = Current;

        fn next(&mut self) -> Option<Self::Item> {
            match self.state {
                Current::None => {
                    self.state = Current::Path(PathBuf::default());
                    Some(self.state.clone())
                }
                Current::Path(_) => {
                    self.state = Current::Confirm(Change::default());
                    Some(self.state.clone())
                }
                Current::Confirm(_) => {
                    self.state = Current::Rescue(Change::default());
                    Some(self.state.clone())
                }
                Current::Rescue(_) => {
                    self.state = Current::Resolving(Confirmation::Accept);
                    Some(self.state.clone())
                }
                Current::Resolving(_) => {
                    self.state = Current::Resolved;
                    Some(self.state.clone())
                }
                Current::Resolved => None,
            }
        }
    }

    #[test]
    fn state_new() {
        let state = State::default();
        assert_eq!(state.len(), 0);

        let state = State::new(0xDEAD);
        assert_eq!(state.index(), 0);
        assert_eq!(state.len(), 0xDEAD);
        assert_eq!(state.current(), &Current::None);
        assert!(state.actions().is_empty());
        assert!(state.logs().is_empty());
    }

    #[test]
    fn set_current_path() {
        let path = PathBuf::from("/test");

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: vec![Action::Cancel],
                ..State::default()
            };
            state.set_current_path(path.clone());

            match current {
                Current::None | Current::Resolved => {
                    assert_eq!(state.current, Current::Path(path.clone()));
                    assert!(state.actions.is_empty());
                }
                _ => {
                    assert_eq!(state.current, current);
                }
            }
        }
    }

    #[test]
    fn set_current_confirm() {
        let path = PathBuf::from("/test");
        let replacement = Replacement::try_from(path.as_path()).unwrap();

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: vec![Action::Cancel],
                ..State::default()
            };
            state.set_current_confirm(replacement.clone(), &[]);

            match current {
                Current::Path(_) | Current::Resolving(_) => {
                    assert_eq!(
                        state.current,
                        Current::Confirm(Change::new(replacement.clone()))
                    );
                    assert!(state
                        .actions
                        .iter()
                        .any(|action| action == &Action::Accept));
                }
                _ => {
                    assert_eq!(state.current, current);
                }
            }
        }
    }

    #[test]
    fn set_current_confirm_with_matchers() {
        let path = PathBuf::from("/test/foo 20240120");
        let mut replacement = Replacement::try_from(path.as_path()).unwrap();
        replacement.new_file_stem = String::from("2024-01-20 foo");
        let matchers = [matchers::ymd_boxed(), matchers::today_boxed()];

        let mut state = State {
            current: Current::Path(PathBuf::default()),
            ..State::default()
        };
        state.set_current_confirm(replacement.clone(), &matchers);

        assert!(matches!(state.current, Current::Confirm(_)));
        let change = state.change().unwrap();
        assert_eq!(change.alternatives.len(), 1);

        let (key, rep) = change.alternatives.iter().next().unwrap();
        assert_eq!(key, crate::matcher::predetermined_date::TODAY);
        assert_eq!(replacement.parent, rep.parent);
    }

    #[test]
    fn set_current_rescue() {
        let replacement = Replacement::default();

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: vec![Action::Cancel],
                ..State::default()
            };
            state.set_current_rescue(replacement.clone());

            match current {
                Current::Path(_) => {
                    assert_eq!(
                        state.current,
                        Current::Rescue(Change::new(replacement.clone()))
                    );
                    assert!(state
                        .actions
                        .iter()
                        .any(|action| action == &Action::Skip));
                }
                _ => {
                    assert_eq!(state.current, current);
                }
            }
        }
    }

    #[test]
    fn set_current_resolving() {
        let conf = Confirmation::Accept;

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: Action::determine_for(&current),
                ..State::default()
            };
            let resolving = state.set_current_resolving(conf.clone());

            match current {
                Current::Confirm(_) => {
                    assert!(resolving);
                    assert_eq!(
                        state.current,
                        Current::Resolving(Confirmation::Accept)
                    );
                    assert!(state.actions.is_empty());
                }
                Current::Rescue(_) => {
                    assert!(!resolving);
                    assert_eq!(state.current, current);

                    let resolving =
                        state.set_current_resolving(Confirmation::Skip);
                    assert!(resolving);
                    assert_eq!(
                        state.current,
                        Current::Resolving(Confirmation::Accept)
                    );
                    assert!(state.actions.is_empty());
                }
                _ => {
                    assert!(!resolving);
                    assert_eq!(state.current, current);
                }
            }
        }
    }

    #[test]
    fn set_current_success() {
        let replacement = Replacement::default();

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: vec![Action::Cancel],
                ..State::default()
            };
            state.set_current_success(replacement.clone());

            match current {
                Current::Resolving(_) => {
                    assert_eq!(state.current, Current::Resolved);
                    assert!(state.actions.is_empty());
                    assert_eq!(state.index(), 1);
                    assert_eq!(
                        state.logs,
                        [ProcessingResult::Success(replacement.clone())]
                    );
                }
                _ => {
                    assert_eq!(state.current, current);
                    assert_eq!(state.index(), 0);
                }
            }
        }
    }

    #[test]
    fn set_current_failure() {
        let path = PathBuf::default();
        let error = String::default();

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: vec![Action::Cancel],
                ..State::default()
            };
            state.set_current_failure(path.clone(), error.clone());

            match current {
                Current::Resolving(_) => {
                    assert_eq!(state.current, Current::Resolved);
                    assert!(state.actions.is_empty());
                    assert_eq!(state.index(), 1);
                    assert_eq!(
                        state.logs,
                        [ProcessingResult::Failure(
                            path.clone(),
                            error.clone()
                        )]
                    );
                }
                _ => {
                    assert_eq!(state.current, current);
                    assert_eq!(state.index(), 0);
                }
            }
        }
    }

    #[test]
    fn customize() {
        let string = String::from("foo");

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: vec![Action::Cancel],
                ..State::default()
            };
            state.customize(string.clone());

            match current {
                Current::Confirm(_) | Current::Rescue(_) => {
                    assert_eq!(
                        state.change().unwrap().customize,
                        Some(string.clone())
                    );
                    assert!(state
                        .actions
                        .iter()
                        .all(|action| action != &Action::Cancel));
                }
                _ => {
                    assert_eq!(state.actions, vec![Action::Cancel]);
                }
            }
        }
    }

    #[test]
    fn cancel_customize() {
        let string = String::from("foo");

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                actions: vec![Action::Cancel],
                ..State::default()
            };
            if let Some(change) = state.change_mut() {
                change.customize = Some(string.clone());
            }
            state.cancel_customize();

            match current {
                Current::Confirm(_) | Current::Rescue(_) => {
                    assert_eq!(state.change().unwrap().customize, None);
                    assert!(state
                        .actions
                        .iter()
                        .all(|action| action != &Action::Cancel));
                }
                _ => {
                    assert_eq!(state.actions, vec![Action::Cancel]);
                }
            }
        }
    }

    #[test]
    fn customized_replacement() {
        for current in CurrentIterator::default() {
            let state = State {
                current: current.clone(),
                ..State::default()
            };

            assert_eq!(state.customized_replacement(), None);
        }

        let string = String::from("foo");

        for current in CurrentIterator::default() {
            let mut state = State {
                current: current.clone(),
                ..State::default()
            };

            if let Some(change) = state.change_mut() {
                change.customize = Some(string.clone());
            }

            match current {
                Current::Confirm(_) | Current::Rescue(_) => {
                    assert!(state.customized_replacement().is_some());
                    assert_eq!(
                        state.customized_replacement().unwrap().new_file_stem,
                        string
                    );
                }
                _ => {
                    assert_eq!(state.customized_replacement(), None);
                }
            }
        }
    }

    #[test]
    fn is_further_customizable() {
        let mut change = Change::default();
        assert!(change.is_further_customizable());

        change.customize = Some(String::default());
        assert!(!change.is_further_customizable());

        change.alternatives =
            HashMap::from([("Hello".to_string(), Replacement::default())]);
        assert!(change.is_further_customizable());
    }
}
