use crate::matcher::Matcher;
use crate::replacement::Replacement;
use crate::ui::actions::Actions;

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Default)]
pub struct State {
    pub index: usize,
    pub len: usize,
    pub current: Current,
    pub actions: Actions,
    pub logs: Vec<ProcessingResult>,
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
        self.current = Current::new_confirm(replacement, matchers);
        self.actions = Actions::from(&self.current);
    }
    pub fn set_current_rescue(&mut self, replacement: Replacement) {
        self.current = Current::new_rescue(replacement);
        self.actions = Actions::from(&self.current);
    }

    pub fn success(&mut self, replacement: Replacement) {
        self.index += 1;
        self.logs.push(ProcessingResult::Success(replacement));
    }
    pub fn failure(&mut self, path: PathBuf, error: String) {
        self.index += 1;
        self.logs.push(ProcessingResult::Failure(path, error));
    }
}

/// Element currently being processed
#[derive(Default)]
pub enum Current {
    #[default]
    None,
    Path(PathBuf),
    Confirm(Change),
    Rescue(Change),
}

impl Current {
    pub fn new_confirm(
        replacement: Replacement,
        matchers: &[Box<dyn Matcher>],
    ) -> Self {
        Self::Confirm(Change::new_confirm(replacement, matchers))
    }

    pub fn new_rescue(replacement: Replacement) -> Self {
        Self::Rescue(Change::new_rescue(replacement))
    }

    pub fn customize(&mut self, string: String) {
        match self {
            Current::Confirm(change) | Current::Rescue(change) => {
                change.customize = Some(string)
            }
            _ => {}
        };
    }

    pub fn customized_replacement(&self) -> Option<Replacement> {
        match &self {
            Current::Confirm(change) | Current::Rescue(change) => {
                if let Some(value) = change.customize.clone() {
                    return Some(
                        change.replacement.clone().new_file_stem(value),
                    );
                }
            }
            _ => {}
        }

        None
    }
}

pub struct Change {
    pub replacement: Replacement,
    pub alternatives: HashMap<String, Replacement>,
    pub customize: Option<String>,
}

impl Change {
    pub fn show_customize_button(&self) -> bool {
        self.customize.is_none() || !self.alternatives.is_empty()
    }

    pub fn new_confirm(
        replacement: Replacement,
        matchers: &[Box<dyn Matcher>],
    ) -> Self {
        let path_buf = replacement.path();
        let path = path_buf.as_path();

        Self {
            alternatives: matchers
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
                .collect(),
            replacement,
            customize: None,
        }
    }

    pub fn new_rescue(replacement: Replacement) -> Self {
        Self {
            replacement,
            alternatives: Default::default(),
            customize: None,
        }
    }
}

#[derive(Clone)]
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
