//! The `[list]` keymap section: in a dialog that is not typing into a text
//! field, a key bound there acts as the arrow / PgUp / PgDn / Home / End /
//! Enter / Esc key it names. Every list handler keeps reading those keys,
//! so one translation here serves them all.

use crate::app::state::{AppState, PopupType};
use crate::keybindings::KeybindingResolver;
use crate::keybindings::screens::ListAction;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// The key the open dialog should see, or `None` when the key only started
/// a sequence of the list keymap.
pub fn translate(
    state: &mut AppState,
    resolver: &mut KeybindingResolver,
    key: KeyEvent,
) -> Option<KeyEvent> {
    let Some(top) = state.dialogs.top_mut() else {
        return Some(key);
    };
    // Typing goes to the field; plugin dialogs match keys themselves.
    if top.focused_field_mut().is_some() || matches!(top, PopupType::Plugin(_)) {
        return Some(key);
    }
    match resolver.list.dispatch(key) {
        Some(action) => Some(KeyEvent {
            code: canonical(action),
            modifiers: KeyModifiers::NONE,
            ..key
        }),
        None if resolver.list.is_ongoing() => None,
        None => Some(key),
    }
}

fn canonical(action: ListAction) -> KeyCode {
    match action {
        ListAction::Up => KeyCode::Up,
        ListAction::Down => KeyCode::Down,
        ListAction::PageUp => KeyCode::PageUp,
        ListAction::PageDown => KeyCode::PageDown,
        ListAction::First => KeyCode::Home,
        ListAction::Last => KeyCode::End,
        ListAction::Activate => KeyCode::Enter,
        ListAction::Close => KeyCode::Esc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use std::path::PathBuf;

    fn resolver(overrides: &[(&str, &str)]) -> KeybindingResolver {
        let mut config = AppConfig::default();
        for (id, keys) in overrides {
            config.keybindings.set_override("all", id, keys);
        }
        KeybindingResolver::new(&config)
    }

    fn press(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn bound_keys_become_arrows_only_outside_text_fields() {
        let mut resolver = resolver(&[("list.down", "j, Down"), ("list.first", "g g, Home")]);
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::Info("x".into()));
        let down = translate(&mut state, &mut resolver, press('j')).unwrap();
        assert_eq!(down.code, KeyCode::Down);
        assert_eq!(translate(&mut state, &mut resolver, press('g')), None);
        let first = translate(&mut state, &mut resolver, press('g')).unwrap();
        assert_eq!(first.code, KeyCode::Home);
        assert_eq!(
            translate(&mut state, &mut resolver, press('x'))
                .unwrap()
                .code,
            KeyCode::Char('x')
        );

        state.dialogs.replace(PopupType::QuickFilterPrompt {
            input: Default::default(),
            original_mask: None,
            original_cursor: 0,
        });
        let typed = translate(&mut state, &mut resolver, press('j')).unwrap();
        assert_eq!(typed.code, KeyCode::Char('j'));
    }
}
