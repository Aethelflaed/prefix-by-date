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
}

impl From<Confirmation> for Action {
    fn from(conf: Confirmation) -> Self {
        match conf {
            Confirmation::Accept => Action::Accept,
            Confirmation::Always => Action::Always,
            Confirmation::Replace(rep) => Action::Replace(rep),
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
        }
    }
}

#[derive(Default)]
pub struct Actions {
    actions: Vec<Action>,
}

impl From<&Current> for Actions {
    fn from(current: &Current) -> Self {
        match current {
            Current::Confirm(change) => Self {
                actions: vec![
                    Action::Accept,
                    Action::Always,
                    Action::Replace(change.replacement.clone()),
                    Action::Customize(change.replacement.clone()),
                    Action::Skip,
                    Action::Refuse,
                    Action::Ignore,
                    Action::Abort,
                ],
            },
            Current::Rescue(change) => Self {
                actions: vec![
                    Action::Replace(change.replacement.clone()),
                    Action::Customize(change.replacement.clone()),
                    Action::Skip,
                    Action::Refuse,
                    Action::Abort,
                ],
            },
            _ => Actions::empty(),
        }
    }
}

impl Actions {
    pub fn empty() -> Self {
        Self { actions: vec![] }
    }

    pub fn all() -> Self {
        Self {
            actions: vec![
                Action::Accept,
                Action::Always,
                Action::Customize(Replacement::default()),
                Action::Skip,
                Action::Refuse,
                Action::Ignore,
                Action::Abort,
            ],
        }
    }

    pub fn find<F>(&self, func: F) -> Option<Action>
    where
        F: Fn(&&Action) -> bool,
    {
        self.actions.iter().find(func).cloned()
    }

    pub fn shortcuts_using<F, T>(&self, func: F) -> Vec<T>
    where
        F: Fn(&Action) -> Option<T>,
    {
        self.actions.iter().filter_map(func).collect()
    }

    pub fn contains(&self, needle: Action) -> bool {
        self.find(|action| {
            std::mem::discriminant(*action) == std::mem::discriminant(&needle)
        })
        .is_some()
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
    }
}
