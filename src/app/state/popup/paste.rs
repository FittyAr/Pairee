use super::PopupType;

impl PopupType {
    /// Append a single-line paste into the focused text field, if any.
    /// Returns true when the overlay consumed the paste.
    pub fn apply_paste(&mut self, paste: &str) -> bool {
        if paste.is_empty() {
            return false;
        }
        match self {
            PopupType::MkDirPrompt {
                input, cursor_idx, ..
            }
            | PopupType::RenamePrompt {
                input, cursor_idx, ..
            } if *cursor_idx == 0 => {
                input.push_str(paste);
                true
            }
            PopupType::CopyPrompt(prompt) | PopupType::MovePrompt(prompt)
                if prompt.cursor_idx == 0 =>
            {
                prompt.input.push_str(paste);
                true
            }
            PopupType::ApplyCommandPrompt { input, .. }
            | PopupType::CompressPrompt { input, .. }
            | PopupType::FilePanelFilterPrompt { input, .. }
            | PopupType::QuickFilterPrompt { input, .. }
            | PopupType::DescribeFilePrompt { input, .. }
            | PopupType::Plugin(crate::app::state::popup::PluginDialog::Input { input, .. }) => {
                input.push_str(paste);
                true
            }
            PopupType::SelectGroupPrompt { query, .. }
            | PopupType::CommandPalette { query, .. }
            | PopupType::WhichKey { query, .. } => {
                query.push_str(paste);
                true
            }
            PopupType::CreateLinkPrompt { dest_input, .. } => {
                dest_input.push_str(paste);
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::types::LinkKind;
    use std::path::PathBuf;

    #[test]
    fn empty_paste_is_ignored() {
        let mut popup = PopupType::ApplyCommandPrompt {
            input: "echo ".into(),
            targets: vec![],
        };
        assert!(!popup.apply_paste(""));
    }

    #[test]
    fn mkdir_pastes_only_when_input_focused() {
        let mut popup = PopupType::MkDirPrompt {
            input: "dir".into(),
            cursor_idx: 0,
            process_multiple: false,
        };
        assert!(popup.apply_paste("_x"));
        match &popup {
            PopupType::MkDirPrompt { input, .. } => assert_eq!(input, "dir_x"),
            _ => panic!("expected mkdir"),
        }

        if let PopupType::MkDirPrompt { cursor_idx, .. } = &mut popup {
            *cursor_idx = 1;
        }
        assert!(!popup.apply_paste("_y"));
        match &popup {
            PopupType::MkDirPrompt { input, .. } => assert_eq!(input, "dir_x"),
            _ => panic!("expected mkdir"),
        }
    }

    #[test]
    fn command_palette_appends_to_query() {
        let mut popup = PopupType::CommandPalette {
            query: "co".into(),
            cursor_idx: 0,
            items: vec![],
        };
        assert!(popup.apply_paste("py"));
        match popup {
            PopupType::CommandPalette { query, .. } => assert_eq!(query, "copy"),
            _ => panic!("expected palette"),
        }
    }

    #[test]
    fn create_link_pastes_into_dest() {
        let mut popup = PopupType::CreateLinkPrompt {
            src: PathBuf::from("a"),
            dest_input: "b".into(),
            kind: LinkKind::Symbolic,
        };
        assert!(popup.apply_paste("/c"));
        match popup {
            PopupType::CreateLinkPrompt { dest_input, .. } => {
                assert_eq!(dest_input, "b/c");
            }
            _ => panic!("expected link prompt"),
        }
    }

    #[test]
    fn which_key_appends_to_query() {
        let mut popup = PopupType::WhichKey {
            query: "F".into(),
            cursor_idx: 0,
            items: vec![],
        };
        assert!(popup.apply_paste("5"));
        match popup {
            PopupType::WhichKey { query, .. } => assert_eq!(query, "F5"),
            _ => panic!("expected which-key"),
        }
    }

    #[test]
    fn confirm_dialog_does_not_consume_paste() {
        let mut popup = PopupType::ConfirmQuit;
        assert!(!popup.apply_paste("nope"));
    }
}
