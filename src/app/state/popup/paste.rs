use super::PopupType;
use crate::app::text_input::TextField;

impl PopupType {
    /// The text field that currently receives typing and pastes, if any.
    pub fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        match self {
            PopupType::MkDirPrompt {
                input,
                cursor_idx: 0,
                ..
            }
            | PopupType::RenamePrompt {
                input,
                cursor_idx: 0,
                ..
            }
            | PopupType::ApplyCommandPrompt { input, .. }
            | PopupType::CompressPrompt { input, .. }
            | PopupType::FilePanelFilterPrompt { input, .. }
            | PopupType::RenameTabPrompt { input, .. }
            | PopupType::QuickFilterPrompt { input, .. }
            | PopupType::PanelSearch { query: input, .. }
            | PopupType::DescribeFilePrompt { input, .. }
            | PopupType::CopyMoveFilterPrompt { input, .. }
            | PopupType::SelectGroupPrompt { query: input, .. }
            | PopupType::CreateLinkPrompt {
                dest_input: input, ..
            } => Some(input),
            PopupType::TransferPrompt(prompt) if prompt.cursor_idx == 0 => Some(&mut prompt.input),
            PopupType::CommandPalette { query, .. } => Some(query),
            PopupType::Shortcuts(s) => match &mut s.mode {
                crate::app::shortcuts::state::Mode::Browse => Some(&mut s.query),
                crate::app::shortcuts::state::Mode::Export { name } => Some(name),
                _ => None,
            },
            PopupType::EditorSearchPrompt(search) | PopupType::ViewerSearchPrompt(search)
                if search.cursor_idx == 0 =>
            {
                Some(&mut search.query)
            }
            PopupType::EditorSaveAsPrompt { input }
            | PopupType::Plugin(super::PluginDialog::Input { input, .. }) => Some(input),
            PopupType::SearchPrompt {
                query,
                content_query,
                cursor_idx,
                ..
            } => match *cursor_idx {
                0 => Some(query),
                1 => Some(content_query),
                _ => None,
            },
            PopupType::MultiRename(dialog) => dialog.focused_field_mut(),
            PopupType::SshConnectPrompt(prompt) => {
                let row = prompt.cursor_idx;
                prompt.field_at(row).map(|(_, field)| field)
            }
            _ => None,
        }
    }

    /// Inserts a single-line paste into the focused text field, if any.
    /// Returns true when the overlay consumed the paste.
    pub fn apply_paste(&mut self, paste: &str) -> bool {
        !paste.is_empty()
            && self
                .focused_field_mut()
                .is_some_and(|field| field.paste(paste))
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
            kind: crate::app::state::popup::CreateKind::Folder,
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
    fn shortcuts_filter_takes_pastes_only_while_browsing() {
        use crate::app::shortcuts::state::{Mode, ShortcutsState};
        let mut s = ShortcutsState::new("norton".into(), Vec::new());
        s.query.set_text("F");
        let mut popup = PopupType::Shortcuts(Box::new(s));
        assert!(popup.apply_paste("5"));
        let PopupType::Shortcuts(s) = &mut popup else {
            panic!("expected shortcuts");
        };
        assert_eq!(s.query.text(), "F5");
        s.mode = Mode::KeySearch;
        assert!(!popup.apply_paste("x"));
    }

    #[test]
    fn confirm_dialog_does_not_consume_paste() {
        let mut popup = PopupType::ConfirmQuit;
        assert!(!popup.apply_paste("nope"));
    }
}
