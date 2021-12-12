use std::{collections::HashMap, fmt::Display, slice::Iter};

use log::warn;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::tui::inputs::key::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum Action {
    // Available everywhere actions
    Quit,
    SwitchToLogWidget,
    SwitchToPreviousWidget,
    SelectDown,
    SelectUp,
    Enter,

    // WidgetSwitching
    SwitchToDirListWidget,

    // TuiLogWidget actions
    LogToggleHideSelector,
    LogToggleFocus,
    LogSelectPreviousTarget,
    LogSelectNextTarget,
    LogReduceShown,
    LogIncreaseShown,
    LogDecreaseCapture,
    LogIncreaseCapture,
    LogPageUp,
    LogPageDown,
    LogExitPageMode,
    LogToggleHideTargets,

    // MetadataWidgetActions
    SaveTagsToFile,
}

impl Action {
    /// All available actions. Now unused and unmaintained
    #[deprecated]
    #[allow(dead_code)]
    pub fn iterator() -> Iter<'static, Action> {
        static ACTIONS: [Action; 2] = [Action::Quit, Action::LogToggleHideSelector];
        ACTIONS.iter()
    }

    pub fn keys(&self) -> &[Key] {
        match self {
            Action::Quit => &[Key::Ctrl('c'), Key::Char('q')],
            Action::LogToggleHideSelector => &[Key::Char('h')],
            Action::LogToggleFocus => &[Key::Char('f')],
            Action::LogSelectPreviousTarget => &[Key::Up],
            Action::LogSelectNextTarget => &[Key::Down],
            Action::LogReduceShown => &[Key::Left],
            Action::LogIncreaseShown => &[Key::Right],
            Action::LogIncreaseCapture => &[Key::Char('+')],
            Action::LogDecreaseCapture => &[Key::Char('-')],
            Action::LogPageUp => &[Key::PageUp],
            Action::LogPageDown => &[Key::PageDown],
            Action::LogExitPageMode => &[Key::Esc],
            Action::LogToggleHideTargets => &[Key::Char(' ')],
            Action::SwitchToLogWidget => &[Key::Ctrl('l')],
            Action::SwitchToPreviousWidget => &[Key::Esc],
            Action::SelectDown => &[Key::Char('j')],
            Action::SelectUp => &[Key::Char('k')],
            Action::Enter => &[Key::Enter],
            Action::SaveTagsToFile => &[Key::Char('s')],
            Action::SwitchToDirListWidget => &[Key::Char('d')],
        }
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Action::Quit => "Quit",
            Action::LogToggleHideSelector => "LogToggleHideSelector",
            Action::LogToggleFocus => "LogToggleFocus",
            Action::LogSelectPreviousTarget => "LogSelectPreviousTarget",
            Action::LogSelectNextTarget => "LogSelectNextTarget",
            Action::LogReduceShown => "LogReduceShown",
            Action::LogIncreaseShown => "LogIncreaseShown",
            Action::LogIncreaseCapture => "LogIncreaseCaptured",
            Action::LogDecreaseCapture => "LogReduceCaptured",
            Action::LogPageUp => "LogPageUp",
            Action::LogPageDown => "LogPageDown",
            Action::LogExitPageMode => "LogExitPageMode",
            Action::LogToggleHideTargets => "LogToggleHideTargets",
            Action::SwitchToLogWidget => "SwitchToLogWidget",
            Action::SwitchToPreviousWidget => "SwitchToPreviousWidget",
            Action::SelectDown => "SelectDown",
            Action::SelectUp => "SelectUp",
            Action::Enter => "EnterKey",
            Action::SaveTagsToFile => "SaveTagsToFile",
            Action::SwitchToDirListWidget => "SwitchToDirListWidget",
        };

        write!(f, "{}", str)
    }
}

/// The application should have contextual actions
#[derive(Default, Debug, Clone)]
pub struct Actions(Vec<Action>);

impl Actions {
    /// Given a key, find the corresponding action
    pub fn find(&self, key: Key) -> Option<Action> {
        // Before, a static array was used. With the strum crate, some boilerplate is removed
        Action::iter()
            .filter(|action| self.0.contains(action))
            .find(|action| action.keys().contains(&key))
    }

    /// Get contextual actions
    pub fn actions(&self) -> &[Action] {
        self.0.as_slice()
    }
}

impl From<Vec<Action>> for Actions {
    fn from(actions: Vec<Action>) -> Self {
        // Check key unicity
        let mut map: HashMap<Key, Vec<Action>> = HashMap::new();
        for action in actions.iter() {
            for key in action.keys().iter() {
                match map.get_mut(key) {
                    Some(vec) => vec.push(*action),
                    None => {
                        map.insert(*key, vec![*action]);
                    }
                }
            }
        }
        let errors = map
            .iter()
            .filter(|(_, actions)| actions.len() > 1) // at least two actions share same shortcut
            .map(|(key, actions)| {
                let actions = actions
                    .iter()
                    .map(Action::to_string)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("Conflict key {} with actions {}", key, actions)
            })
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            warn!("{}", errors.join("; "))
        }

        // Ok, we can create contextual actions
        Self(actions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_find_action_by_key() {
        let actions: Actions = vec![Action::Quit].into();
        let result = actions.find(Key::Ctrl('c'));
        assert_eq!(result, Some(Action::Quit));
    }

    #[test]
    fn should_find_action_by_key_not_found() {
        let actions: Actions = vec![Action::Quit].into();
        let result = actions.find(Key::Alt('w'));
        assert_eq!(result, None);
    }

    #[test]
    fn should_create_actions_from_vec() {
        let _actions: Actions = vec![Action::Quit].into();
    }

    /*
    #[test]
    #[should_panic]
    fn should_panic_when_create_actions_conflict_key() {
        let _actions: Actions =
            vec![Action::Quit, Action::Quit, Action::LogToggleHideTargets].into();
    }
    */
}
