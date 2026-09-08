use crate::app::state::AppState;

/// First non-empty line of a bracketed-paste payload (`\r` stripped).
pub fn first_paste_line(raw: &str) -> String {
    raw.replace('\r', "")
        .lines()
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Insert a paste into the focused text field, or the CLI when no overlay is open.
pub fn handle_paste(state: &mut AppState, raw: &str) {
    let line = first_paste_line(raw);
    if line.is_empty() {
        return;
    }
    if state.dialogs.is_some() {
        if let Some(popup) = state.dialogs.top_mut() {
            let _ = popup.apply_paste(&line);
        }
        return;
    }
    state.cli_input.push_str(&line);
}

#[cfg(test)]
mod paste_tests {
    use super::*;
    use crate::app::state::PopupType;
    use std::path::PathBuf;

    #[test]
    fn first_paste_line_strips_cr_and_extra_lines() {
        assert_eq!(first_paste_line("hello\r\nworld"), "hello");
        assert_eq!(first_paste_line("\n\nfoo"), "foo");
        assert_eq!(first_paste_line(""), "");
    }

    #[test]
    fn paste_goes_to_cli_when_no_dialog() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        handle_paste(&mut state, "echo hi\r\nignored");
        assert_eq!(state.cli_input, "echo hi");
    }

    #[test]
    fn paste_goes_to_rename_prompt() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::RenamePrompt {
            input: "old".into(),
            original: "old".into(),
            src_path: PathBuf::from("old"),
            parent_dir: PathBuf::from("."),
            cursor_idx: 0,
        });
        handle_paste(&mut state, "new-name.txt\n");
        match state.dialogs.top() {
            Some(PopupType::RenamePrompt { input, .. }) => {
                assert_eq!(input, "oldnew-name.txt");
            }
            other => panic!("expected rename prompt, got {other:?}"),
        }
        assert!(state.cli_input.is_empty());
    }

    #[test]
    fn paste_ignored_on_non_text_dialog() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::ConfirmQuit);
        handle_paste(&mut state, "should-not-land-in-cli");
        assert!(state.cli_input.is_empty());
    }

    #[test]
    fn paste_goes_to_apply_command() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.dialogs.replace(PopupType::ApplyCommandPrompt {
            input: "echo ".into(),
            targets: vec![],
        });
        handle_paste(&mut state, "%f\r\nmore");
        match state.dialogs.top() {
            Some(PopupType::ApplyCommandPrompt { input, .. }) => {
                assert_eq!(input, "echo %f");
            }
            other => panic!("expected apply prompt, got {other:?}"),
        }
    }
}
