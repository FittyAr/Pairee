//! Keyboard handler for the first-run keymap picker.

use crate::app::context::AppContext;
use crate::app::list_nav::{ListKey, ListKeys, list_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crate::ui::popup::onboarding::PRESET_IDS;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::OnboardingKeymap { cursor_idx }) = state.dialogs.top_mut() else {
        return Err(());
    };
    // Tab / Shift+Tab move like Down / Up.
    let code = match key.code {
        KeyCode::Tab => KeyCode::Down,
        KeyCode::BackTab => KeyCode::Up,
        other => other,
    };
    let choice = match list_key(ListKeys::ARROWS, code, cursor_idx, PRESET_IDS.len()) {
        ListKey::Moved => {
            state.mark_ui_dirty();
            return Ok(None);
        }
        ListKey::Activate(idx) => Some(PRESET_IDS[idx.min(PRESET_IDS.len() - 1)]),
        ListKey::Close => None,
        ListKey::Other => return Ok(None),
    };
    apply_choice(context, state, choice);
    context.config.save_logging();
    Ok(None)
}

/// Apply the chosen preset (or keep the current one) and mark onboarding done.
/// Does not persist; the caller saves.
pub fn apply_choice(context: &mut AppContext, state: &mut AppState, preset: Option<&str>) {
    if let Some(name) = preset {
        context.config.keybindings.preset = name.to_string();
        context.config.settings.keybinding_preset = name.to_string();
        context.resolver = crate::keybindings::KeybindingResolver::new(&context.config);
    }
    context.config.settings.onboarding_completed = true;
    state.dialogs.clear();
    state.mark_ui_dirty();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    use std::path::PathBuf;

    fn ctx() -> AppContext {
        AppContext::new(AppConfig::default())
    }

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn down_wraps_through_presets() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let mut context = ctx();
        state
            .dialogs
            .replace(PopupType::OnboardingKeymap { cursor_idx: 0 });
        handle(&mut state, make_key(KeyCode::Down), &mut context).unwrap();
        match state.dialogs.top() {
            Some(PopupType::OnboardingKeymap { cursor_idx }) => assert_eq!(*cursor_idx, 1),
            _ => panic!("expected onboarding"),
        }
        handle(&mut state, make_key(KeyCode::Down), &mut context).unwrap();
        handle(&mut state, make_key(KeyCode::Down), &mut context).unwrap();
        match state.dialogs.top() {
            Some(PopupType::OnboardingKeymap { cursor_idx }) => assert_eq!(*cursor_idx, 0),
            _ => panic!("expected onboarding"),
        }
    }

    #[test]
    fn apply_choice_sets_neovim_and_completes() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let mut context = ctx();
        state
            .dialogs
            .replace(PopupType::OnboardingKeymap { cursor_idx: 1 });
        apply_choice(&mut context, &mut state, Some("neovim"));
        assert!(context.config.settings.onboarding_completed);
        assert_eq!(context.config.settings.keybinding_preset, "neovim");
        assert_eq!(context.config.keybindings.preset, "neovim");
        assert!(state.dialogs.is_none());
    }

    #[test]
    fn apply_choice_none_keeps_norton() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let mut context = ctx();
        apply_choice(&mut context, &mut state, None);
        assert!(context.config.settings.onboarding_completed);
        assert_eq!(context.config.settings.keybinding_preset, "norton");
    }
}
