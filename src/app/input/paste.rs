use crate::app::state::AppState;

use crate::app::text_input::first_paste_line;

/// Insert a paste into the focused text field, the editor screen (every
/// line), or the CLI when no overlay is open.
pub fn handle_paste(state: &mut AppState, raw: &str) {
    if state.dialogs.is_empty()
        && (crate::app::screen_input::editor::paste_into_editor(state, raw)
            || crate::app::screen_input::terminal::paste_into_terminal(state, raw))
    {
        return;
    }
    let line = first_paste_line(raw);
    if line.is_empty() {
        return;
    }
    if state.dialogs.is_open() {
        if let Some(popup) = state.dialogs.top_mut() {
            let _ = popup.apply_paste(&line);
        }
        return;
    }
    state.cli.field.insert_str(&line);
}

#[cfg(test)]
mod paste_tests {
    use super::*;
    use crate::app::state::PopupType;
    use crate::app::state::popup::ApplyCommandPrompt;
    use crate::app::state::popup::ConfirmQuit;
    use crate::app::state::popup::RenamePrompt;
    use std::path::PathBuf;

    #[test]
    fn paste_goes_to_cli_when_no_dialog() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        handle_paste(&mut state, "echo hi\r\nignored");
        assert_eq!(state.cli.text(), "echo hi");
    }

    #[test]
    fn paste_goes_to_rename_prompt() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(RenamePrompt {
            input: "old".into(),
            original: "old".into(),
            src_path: PathBuf::from("old"),
            parent_dir: PathBuf::from("."),
            cursor_idx: 0,
        });
        handle_paste(&mut state, "new-name.txt\n");
        match state.dialogs.top() {
            Some(PopupType::RenamePrompt(RenamePrompt { input, .. })) => {
                assert_eq!(input, "oldnew-name.txt");
            }
            other => panic!("expected rename prompt, got {other:?}"),
        }
        assert!(state.cli.field.is_empty());
    }

    #[test]
    fn paste_ignored_on_non_text_dialog() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(ConfirmQuit);
        handle_paste(&mut state, "should-not-land-in-cli");
        assert!(state.cli.field.is_empty());
    }

    #[test]
    fn paste_goes_to_apply_command() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(ApplyCommandPrompt {
            input: "echo ".into(),
            targets: vec![],
        });
        handle_paste(&mut state, "%f\r\nmore");
        match state.dialogs.top() {
            Some(PopupType::ApplyCommandPrompt(ApplyCommandPrompt { input, .. })) => {
                assert_eq!(input, "echo %f");
            }
            other => panic!("expected apply prompt, got {other:?}"),
        }
    }
}
