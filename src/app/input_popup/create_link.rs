use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::{AppState, LinkKind, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::CreateLinkPrompt {
        dest_input, kind, ..
    }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
        *kind = match kind {
            LinkKind::Symbolic => LinkKind::Hard,
            LinkKind::Hard => LinkKind::Symbolic,
        };
        return Ok(None);
    }
    match field_key(dest_input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(PopupType::CreateLinkPrompt {
                src,
                dest_input,
                kind,
            }) = state.dialogs.pop()
            {
                create(state, context, &src, dest_input.text(), kind);
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}

/// Creates the link in the passive panel's folder.
fn create(
    state: &mut AppState,
    context: &AppContext,
    src: &std::path::Path,
    name: &str,
    kind: LinkKind,
) {
    let dest = state.get_passive_panel().current_path.join(name);
    state.dialogs.clear();
    let result = match kind {
        LinkKind::Symbolic => crate::fs::create_symlink(src, &dest),
        LinkKind::Hard => crate::fs::create_hardlink(src, &dest),
    };
    match result {
        Err(e) => state
            .dialogs
            .replace(PopupType::Error(format!("Link failed: {}", e))),
        Ok(_) => state.refresh_both_panels(context.config.settings.show_hidden),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crossterm::event::KeyModifiers;
    use std::path::PathBuf;

    fn press(state: &mut AppState, code: KeyCode) {
        let mut context = AppContext::new(AppConfig {
            settings: crate::config::settings::Settings::default(),
            theme: crate::config::theme::Theme::default(),
            keybindings: crate::config::keybindings::KeybindingsConfig::default(),
        });
        let _ = handle(state, KeyEvent::new(code, KeyModifiers::NONE), &mut context);
    }

    #[test]
    fn s_and_h_are_typed_and_tab_switches_kind() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::CreateLinkPrompt {
            src: PathBuf::from("a"),
            dest_input: Default::default(),
            kind: LinkKind::Symbolic,
        });
        for c in "hosts".chars() {
            press(&mut state, KeyCode::Char(c));
        }
        press(&mut state, KeyCode::Tab);
        match state.dialogs.top() {
            Some(PopupType::CreateLinkPrompt {
                dest_input, kind, ..
            }) => {
                assert_eq!(dest_input, "hosts");
                assert_eq!(*kind, LinkKind::Hard);
            }
            other => panic!("expected link prompt, got {other:?}"),
        }
    }
}
