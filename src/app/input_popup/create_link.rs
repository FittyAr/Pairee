use crate::app::context::AppContext;
use crate::app::state::{AppState, LinkKind, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::CreateLinkPrompt {
        src,
        dest_input,
        kind,
    }) = state.dialogs.top().cloned()
    {
        match key.code {
            KeyCode::Tab | KeyCode::BackTab => {
                let new_kind = match kind {
                    LinkKind::Symbolic => LinkKind::Hard,
                    LinkKind::Hard => LinkKind::Symbolic,
                };
                state.dialogs.replace(PopupType::CreateLinkPrompt {
                    src,
                    dest_input,
                    kind: new_kind,
                });
                return Ok(None);
            }
            KeyCode::Char(c) => {
                let mut new_input = dest_input;
                new_input.push(c);
                state.dialogs.replace(PopupType::CreateLinkPrompt {
                    src,
                    dest_input: new_input,
                    kind,
                });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_input = dest_input;
                new_input.pop();
                state.dialogs.replace(PopupType::CreateLinkPrompt {
                    src,
                    dest_input: new_input,
                    kind,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                let dest = state.get_passive_panel().current_path.join(&dest_input);
                state.dialogs.clear();
                let result = match kind {
                    LinkKind::Symbolic => crate::fs::create_symlink(&src, &dest),
                    LinkKind::Hard => crate::fs::create_hardlink(&src, &dest),
                };
                if let Err(e) = result {
                    state
                        .dialogs
                        .replace(PopupType::Error(format!("Link failed: {}", e)));
                } else {
                    state.refresh_both_panels(context.config.settings.show_hidden);
                }
                return Ok(None);
            }
            KeyCode::Esc => {
                state.dialogs.clear();
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
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
            dest_input: String::new(),
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
