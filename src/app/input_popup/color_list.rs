//! Shared key handling of the two color-list dialogs reached from the
//! configuration "Colors" tab: theme color groups and file highlighting
//! rules. Each row holds a color name; Left / Right cycle named colors,
//! Enter types one, `S` saves, Esc goes back.

use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, ConfigurationDialogState, PopupType};
use crate::app::text_input::TextField;
use crate::config::AppConfig;
use crate::config::theme::{COLOR_PROPS, Theme, cycle_named_color};
use crate::ui::highlight::HighlightRule;
use crossterm::event::{KeyCode, KeyEvent};

/// Configuration dialog tab holding the color dialogs.
const COLORS_TAB: usize = 6;

/// A list of editable colors (Strategy over theme fields / highlight rules).
pub trait ColorList {
    fn len(&self) -> usize;
    fn color_mut(&mut self, idx: usize) -> Option<&mut String>;
}

impl ColorList for Theme {
    fn len(&self) -> usize {
        COLOR_PROPS.len()
    }

    fn color_mut(&mut self, idx: usize) -> Option<&mut String> {
        COLOR_PROPS.get(idx).map(|prop| (prop.get_mut)(self))
    }
}

impl ColorList for Vec<HighlightRule> {
    fn len(&self) -> usize {
        self.as_slice().len()
    }

    fn color_mut(&mut self, idx: usize) -> Option<&mut String> {
        self.get_mut(idx).map(|rule| &mut rule.color)
    }
}

/// What the dialog must do after a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorListKey {
    Handled,
    /// `S`: apply and save, then go back.
    Save,
    /// Esc (not typing): back to the configuration dialog.
    Back,
}

pub fn handle_key<L: ColorList>(
    list: &mut L,
    cursor: &mut usize,
    edit: &mut Option<TextField>,
    key: &KeyEvent,
) -> ColorListKey {
    if let Some(field) = edit {
        match key.code {
            KeyCode::Esc => *edit = None,
            KeyCode::Enter => {
                if let Some(color) = list.color_mut(*cursor) {
                    *color = field.text().to_string();
                }
                *edit = None;
            }
            _ => {
                field.handle_key(key);
            }
        }
        return ColorListKey::Handled;
    }
    match list_key(ListKeys::ARROWS, key.code, cursor, list.len()) {
        ListKey::Close => return ColorListKey::Back,
        ListKey::Activate(idx) => {
            *edit = list.color_mut(idx).map(|c| TextField::new(c.as_str()));
        }
        ListKey::Other => match key.code {
            KeyCode::Char('s' | 'S') => return ColorListKey::Save,
            KeyCode::Left | KeyCode::Right => {
                if let Some(color) = list.color_mut(*cursor) {
                    *color = cycle_named_color(color, key.code == KeyCode::Right);
                }
            }
            _ => {}
        },
        ListKey::Moved => {}
    }
    ColorListKey::Handled
}

/// Back to the configuration dialog's Colors tab, on row `row`.
pub fn back_to_config(state: &mut AppState, config: &AppConfig, row: usize) {
    state.dialogs.replace(PopupType::ConfigurationDialog(
        ConfigurationDialogState::new(config, COLORS_TAB, row, false),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn cycle_type_and_save() {
        let mut rules = vec![HighlightRule {
            mask: "*.rs".into(),
            color: "Red".into(),
        }];
        let (mut cursor, mut edit) = (0, None);
        handle_key(&mut rules, &mut cursor, &mut edit, &key(KeyCode::Right));
        assert_eq!(rules[0].color, "Green");
        handle_key(&mut rules, &mut cursor, &mut edit, &key(KeyCode::Enter));
        assert!(edit.is_some());
        handle_key(&mut rules, &mut cursor, &mut edit, &key(KeyCode::Backspace));
        handle_key(&mut rules, &mut cursor, &mut edit, &key(KeyCode::Char('X')));
        handle_key(&mut rules, &mut cursor, &mut edit, &key(KeyCode::Enter));
        assert_eq!(rules[0].color, "GreeX");
        assert_eq!(
            handle_key(&mut rules, &mut cursor, &mut edit, &key(KeyCode::Char('s'))),
            ColorListKey::Save
        );
        assert_eq!(
            handle_key(&mut rules, &mut cursor, &mut edit, &key(KeyCode::Esc)),
            ColorListKey::Back
        );
    }
}
