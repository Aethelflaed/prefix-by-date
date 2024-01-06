use crate::processing::Confirmation;
use crate::replacement::Replacement;

use crate::ui::state::Current;

#[derive(Debug, Clone)]
pub enum Action {
    Accept,
    Always,
    Skip,
    Refuse,
    Ignore,
    Abort,
    Replace(Replacement),
    Customize(Replacement),
    ViewAlternatives,
    Cancel,
}

impl PartialEq for Action {
    fn eq(&self, other: &Action) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl From<&Confirmation> for Action {
    fn from(conf: &Confirmation) -> Self {
        match conf {
            Confirmation::Accept => Action::Accept,
            Confirmation::Always => Action::Always,
            Confirmation::Replace(rep) => Action::Replace(rep.clone()),
            Confirmation::Skip => Action::Skip,
            Confirmation::Refuse => Action::Refuse,
            Confirmation::Ignore => Action::Ignore,
            Confirmation::Abort => Action::Abort,
        }
    }
}

impl TryInto<Confirmation> for Action {
    type Error = ();

    fn try_into(self) -> std::result::Result<Confirmation, Self::Error> {
        match self {
            Action::Accept => Ok(Confirmation::Accept),
            Action::Always => Ok(Confirmation::Always),
            Action::Replace(rep) => Ok(Confirmation::Replace(rep)),
            Action::Skip => Ok(Confirmation::Skip),
            Action::Refuse => Ok(Confirmation::Refuse),
            Action::Ignore => Ok(Confirmation::Ignore),
            Action::Abort => Ok(Confirmation::Abort),
            Action::Customize(_) => Err(()),
            Action::ViewAlternatives => Err(()),
            Action::Cancel => Err(()),
        }
    }
}

impl Action {
    pub fn determine_for(current: &Current) -> Vec<Self> {
        match current {
            Current::Confirm(change) => {
                let mut actions = vec![Action::Accept, Action::Always];
                if !change.alternatives.is_empty() {
                    actions.push(Action::ViewAlternatives);
                }
                if change.is_further_customizable() {
                    actions.push(Action::Customize(change.replacement.clone()));
                }
                actions.extend_from_slice(&vec![
                    Action::Replace(change.replacement.clone()),
                    Action::Skip,
                    Action::Refuse,
                    Action::Ignore,
                    Action::Abort,
                ]);

                actions
            }
            Current::Rescue(change) => {
                let mut actions = vec![];
                if change.is_further_customizable() {
                    actions.push(Action::Customize(change.replacement.clone()));
                }
                actions.extend_from_slice(&vec![
                    Action::Replace(change.replacement.clone()),
                    Action::Skip,
                    Action::Refuse,
                    Action::Abort,
                ]);

                actions
            }
            _ => vec![],
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Action::Accept,
            Action::Always,
            Action::Customize(Replacement::default()),
            Action::Replace(Replacement::default()),
            Action::Skip,
            Action::Refuse,
            Action::Ignore,
            Action::Abort,
            Action::ViewAlternatives,
            Action::Cancel,
        ]
    }
}

#[allow(dead_code)]
pub fn shortcut_for(action: &Action) -> Option<char> {
    match action {
        Action::Accept => Some('Y'),
        Action::Always => Some('A'),
        Action::Customize(_) => Some('C'),
        Action::Skip => Some('S'),
        Action::Refuse => Some('R'),
        Action::Ignore => Some('I'),
        Action::Abort => Some('Q'),
        Action::Replace(_) => None,
        Action::ViewAlternatives => Some('V'),
        Action::Cancel => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn actions_from_current_none() {
        let current = Current::None;
        assert!(Action::determine_for(&current).is_empty());
    }

    #[test]
    fn actions_from_current_path() {
        use std::path::PathBuf;

        let current = Current::Path(PathBuf::from("hello"));
        assert!(Action::determine_for(&current).is_empty());
    }

    #[test]
    fn actions_from_current_confirm() {
        use crate::ui::state::Change;

        let change = Change::default();
        let current = Current::Confirm(change);
        let actions = Action::determine_for(&current);

        assert!(!actions.contains(&Action::ViewAlternatives));
        assert!(actions.contains(&Action::Customize(Replacement::default())));

        assert_eq!(actions[0], Action::Accept);
        assert_eq!(actions[1], Action::Always);
        assert_eq!(actions[2], Action::Customize(Replacement::default()));
        assert_eq!(actions[3], Action::Replace(Replacement::default()));
        assert_eq!(actions[4], Action::Skip);
        assert_eq!(actions[5], Action::Refuse);
        assert_eq!(actions[6], Action::Ignore);
        assert_eq!(actions[7], Action::Abort);
    }

    #[test]
    fn actions_from_current_confirm_customized() {
        use crate::ui::state::Change;

        let change = Change {
            customize: Some(String::from("foo")),
            ..Change::default()
        };
        let current = Current::Confirm(change);
        let actions = Action::determine_for(&current);

        assert!(!actions.contains(&Action::ViewAlternatives));
        assert!(!actions.contains(&Action::Customize(Replacement::default())));

        assert_eq!(actions[0], Action::Accept);
        assert_eq!(actions[1], Action::Always);
        assert_eq!(actions[2], Action::Replace(Replacement::default()));
        assert_eq!(actions[3], Action::Skip);
        assert_eq!(actions[4], Action::Refuse);
        assert_eq!(actions[5], Action::Ignore);
        assert_eq!(actions[6], Action::Abort);
    }

    #[test]
    fn actions_from_current_confirm_with_alternatives() {
        use crate::ui::state::Change;
        use std::collections::HashMap;

        let change = Change {
            alternatives: HashMap::from([(
                "test".to_string(),
                Replacement::default(),
            )]),
            ..Change::default()
        };
        let current = Current::Confirm(change);
        let actions = Action::determine_for(&current);

        assert!(actions.contains(&Action::ViewAlternatives));
        assert!(actions.contains(&Action::Customize(Replacement::default())));

        assert_eq!(actions[0], Action::Accept);
        assert_eq!(actions[1], Action::Always);
        assert_eq!(actions[2], Action::ViewAlternatives);
        assert_eq!(actions[3], Action::Customize(Replacement::default()));
        assert_eq!(actions[4], Action::Replace(Replacement::default()));
        assert_eq!(actions[5], Action::Skip);
        assert_eq!(actions[6], Action::Refuse);
        assert_eq!(actions[7], Action::Ignore);
        assert_eq!(actions[8], Action::Abort);
    }

    #[test]
    fn actions_from_current_confirm_customized_and_with_alternatives() {
        use crate::ui::state::Change;
        use std::collections::HashMap;

        let change = Change {
            alternatives: HashMap::from([(
                "test".to_string(),
                Replacement::default(),
            )]),
            customize: Some(String::from("foo")),
            ..Change::default()
        };
        let current = Current::Confirm(change);
        let actions = Action::determine_for(&current);

        assert!(actions.contains(&Action::ViewAlternatives));
        assert!(actions.contains(&Action::Customize(Replacement::default())));
    }

    #[test]
    fn actions_from_current_rescue() {
        use crate::ui::state::Change;

        let change = Change::default();
        let current = Current::Rescue(change);
        let actions = Action::determine_for(&current);

        assert!(actions.contains(&Action::Customize(Replacement::default())));

        assert_eq!(actions[0], Action::Customize(Replacement::default()));
        assert_eq!(actions[1], Action::Replace(Replacement::default()));
        assert_eq!(actions[2], Action::Skip);
        assert_eq!(actions[3], Action::Refuse);
        assert_eq!(actions[4], Action::Abort);
    }

    #[test]
    fn actions_from_current_rescue_customized() {
        use crate::ui::state::Change;

        let change = Change {
            customize: Some(String::from("foo")),
            ..Change::default()
        };
        let current = Current::Rescue(change);
        let actions = Action::determine_for(&current);

        assert!(!actions.contains(&Action::Customize(Replacement::default())));

        assert_eq!(actions[0], Action::Replace(Replacement::default()));
        assert_eq!(actions[1], Action::Skip);
        assert_eq!(actions[2], Action::Refuse);
        assert_eq!(actions[3], Action::Abort);
    }

    #[test]
    fn all() {
        let actions = Action::all();

        assert_eq!(actions[0], Action::Accept);
        assert_eq!(actions[1], Action::Always);
        assert_eq!(actions[2], Action::Customize(Replacement::default()));
        assert_eq!(actions[3], Action::Replace(Replacement::default()));
        assert_eq!(actions[4], Action::Skip);
        assert_eq!(actions[5], Action::Refuse);
        assert_eq!(actions[6], Action::Ignore);
        assert_eq!(actions[7], Action::Abort);
    }

    #[test]
    fn iter() {
        let actions = Action::all();

        assert_eq!(
            actions.iter().filter_map(shortcut_for).collect::<Vec<_>>(),
            vec!['Y', 'A', 'C', 'S', 'R', 'I', 'Q', 'V']
        );

        let func = |action: &Action| match shortcut_for(action) {
            Some(c) => Some(c),
            None => Some('?'),
        };

        assert_eq!(
            actions.iter().filter_map(func).collect::<Vec<_>>(),
            vec!['Y', 'A', 'C', '?', 'S', 'R', 'I', 'Q', 'V', '?']
        );
    }

    #[test]
    fn from_confirmation() {
        assert_eq!(Action::Accept, Action::from(&Confirmation::Accept));
        assert_eq!(Action::Always, Action::from(&Confirmation::Always));
        assert_eq!(
            Action::Replace(Replacement::default()),
            Action::from(&Confirmation::Replace(Replacement::default()))
        );
        assert_eq!(Action::Skip, Action::from(&Confirmation::Skip));
        assert_eq!(Action::Refuse, Action::from(&Confirmation::Refuse));
        assert_eq!(Action::Ignore, Action::from(&Confirmation::Ignore));
        assert_eq!(Action::Abort, Action::from(&Confirmation::Abort));
    }

    #[test]
    fn try_into_confirmation() {
        assert_eq!(Confirmation::Accept, Action::Accept.try_into().unwrap());
        assert_eq!(Confirmation::Always, Action::Always.try_into().unwrap());
        assert_eq!(
            Confirmation::Replace(Replacement::default()),
            Action::Replace(Replacement::default()).try_into().unwrap()
        );
        assert_eq!(Confirmation::Skip, Action::Skip.try_into().unwrap());
        assert_eq!(Confirmation::Refuse, Action::Refuse.try_into().unwrap());
        assert_eq!(Confirmation::Ignore, Action::Ignore.try_into().unwrap());
        assert_eq!(Confirmation::Abort, Action::Abort.try_into().unwrap());
        assert_eq!(
            Err(()),
            TryInto::<Confirmation>::try_into(Action::Customize(
                Replacement::default()
            ))
        );
        assert_eq!(
            Err(()),
            TryInto::<Confirmation>::try_into(Action::ViewAlternatives)
        );
        assert_eq!(Err(()), TryInto::<Confirmation>::try_into(Action::Cancel));
    }

    #[test]
    fn find() {
        let actions = Action::all();

        assert_eq!(
            Some(Action::Abort),
            actions
                .iter()
                .find(|act| { shortcut_for(act) == Some('Q') })
                .cloned()
        );
    }
}
