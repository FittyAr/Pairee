//! Opening, saving and reloading editor screens from application state.
//!
//! Every "edit" entry point (F4 on a panel, the viewer, the user menu file,
//! …) goes through [`open_in_editor`]; the editor key handler and its popups
//! use the save/reload helpers so the checks (read-only, changed on disk,
//! existing "save as" target) live in one place.

use super::document::LoadError;
use super::{EditorOptions, EditorState};
use crate::app::state::{AppState, PopupType, Screen};
use crate::config::localization::t;
use crate::config::settings::Settings;
use std::path::{Path, PathBuf};

/// Why saving needs the user's confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwriteReason {
    /// The file was modified by another program since it was opened.
    ChangedOnDisk,
    /// "Save as" target already exists.
    TargetExists,
}

impl OverwriteReason {
    pub fn message_key(self) -> &'static str {
        match self {
            Self::ChangedOnDisk => "editor_changed_on_disk",
            Self::TargetExists => "editor_target_exists",
        }
    }
}

/// Localized description of a load failure.
pub fn load_error_message(err: &LoadError) -> String {
    match err {
        LoadError::Io(e) => format!("{} {}", t("error_read_file_failed"), e),
        LoadError::TooLarge(bytes) => {
            t("editor_error_too_large").replacen("{}", &(bytes / (1024 * 1024)).to_string(), 1)
        }
        LoadError::NotUtf8 => t("editor_error_not_utf8"),
    }
}

impl AppState {
    /// The editor of the active screen, if the active screen is an editor.
    pub fn active_editor_mut(&mut self) -> Option<&mut EditorState> {
        match self.screens.get_mut(self.active_screen_idx) {
            Some(Screen::Editor(ed)) => Some(ed),
            _ => None,
        }
    }
}

/// Opens `path` in the built-in editor, or shows why it cannot be edited.
pub fn open_in_editor(state: &mut AppState, path: PathBuf, settings: &Settings) {
    match EditorState::open(path, &EditorOptions::from(settings)) {
        Ok(mut editor) => {
            if settings.editor_cursor_at_end {
                editor.move_doc_end();
            }
            let read_only = editor.stamp.read_only;
            let locked = editor.locked;
            state.push_screen(Screen::Editor(editor));
            if read_only && (locked || settings.editor_warn_opening_readonly) {
                let key = if locked {
                    "editor_read_only_locked"
                } else {
                    "editor_read_only_warning"
                };
                state.dialogs.replace(PopupType::Info(t(key)));
            }
        }
        Err(e) => {
            state
                .dialogs
                .replace(PopupType::Error(load_error_message(&e)));
        }
    }
}

/// Saves the active editor to `target` (`None` = its own path).
///
/// Unless `force` is set, asks before overwriting a file that changed on
/// disk or an existing "save as" target. Returns `true` when written.
pub fn save_active_editor(state: &mut AppState, target: Option<PathBuf>, force: bool) -> bool {
    let Some(ed) = state.active_editor_mut() else {
        return false;
    };
    let target = target.unwrap_or_else(|| ed.path.clone());
    let reason = if force {
        None
    } else if target == ed.path {
        ed.changed_on_disk()
            .then_some(OverwriteReason::ChangedOnDisk)
    } else {
        target.exists().then_some(OverwriteReason::TargetExists)
    };
    if let Some(reason) = reason {
        state
            .dialogs
            .replace(PopupType::EditorConfirmOverwrite { target, reason });
        return false;
    }
    match ed.save_to(&target) {
        Ok(()) => true,
        Err(e) => {
            state.dialogs.replace(PopupType::Error(
                t("error_save_failed").replace("{}", &e.to_string()),
            ));
            false
        }
    }
}

/// Re-reads the active editor's file from disk.
pub fn reload_active_editor(state: &mut AppState) {
    let Some(ed) = state.active_editor_mut() else {
        return;
    };
    if let Err(e) = ed.reload() {
        state.dialogs.replace(PopupType::Error(
            t("error_reload_file_failed").replace("{}", &load_error_message(&e)),
        ));
    }
}

/// Resolves a "save as" input relative to the edited file's directory.
pub fn resolve_save_as_path(current: &Path, input: &str) -> Option<PathBuf> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(input);
    Some(if candidate.is_absolute() {
        candidate
    } else {
        current
            .parent()
            .map_or_else(|| candidate.clone(), |dir| dir.join(&candidate))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_file(content: &str) -> (AppState, tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("doc.txt");
        std::fs::write(&path, content).unwrap();
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        open_in_editor(&mut state, path.clone(), &Settings::default());
        (state, dir, path)
    }

    #[test]
    fn save_as_existing_target_asks_first() {
        let (mut state, dir, _) = state_with_file("x");
        let other = dir.path().join("other.txt");
        std::fs::write(&other, "keep").unwrap();
        assert!(!save_active_editor(&mut state, Some(other.clone()), false));
        assert!(matches!(
            state.dialogs.top(),
            Some(PopupType::EditorConfirmOverwrite {
                reason: OverwriteReason::TargetExists,
                ..
            })
        ));
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "keep");
        assert!(save_active_editor(&mut state, Some(other.clone()), true));
        assert_eq!(std::fs::read_to_string(&other).unwrap(), "x");
        assert_eq!(state.active_editor_mut().unwrap().path, other);
    }

    #[test]
    fn non_utf8_file_is_not_opened() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.bin");
        std::fs::write(&path, [0xc3, 0x28]).unwrap();
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        open_in_editor(&mut state, path, &Settings::default());
        assert!(state.active_editor_mut().is_none());
        assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
    }

    #[test]
    fn resolve_save_as_is_relative_to_file() {
        let base = Path::new("/a/b/file.txt");
        assert_eq!(
            resolve_save_as_path(base, "new.txt"),
            Some(PathBuf::from("/a/b/new.txt"))
        );
        assert_eq!(resolve_save_as_path(base, "  "), None);
    }
}
