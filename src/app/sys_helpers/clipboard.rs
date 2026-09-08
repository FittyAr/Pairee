//! System clipboard via `arboard` (replaces shelling out to clip/xclip/wl-copy).

use crate::app::state::PanelState;
use std::path::{Path, PathBuf};

/// Set the OS clipboard to `text`.
///
/// Returns `Err` when the session has no clipboard (headless CI, missing
/// Wayland/X11) so callers can show an error instead of failing silently.
pub fn set_text(text: &str) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text.to_owned()))
        .map_err(|e| e.to_string())
}

/// Paths that Copy Path should put on the clipboard.
///
/// Tagged files keep `selection_order`. Otherwise the hovered entry is used
/// (same as other file ops). If nothing is targetable (`..` / empty panel),
/// the panel's current directory is copied.
pub fn paths_to_copy(panel: &PanelState) -> Vec<PathBuf> {
    if !panel.selected_paths.is_empty() {
        let ordered: Vec<PathBuf> = panel
            .selection_order
            .iter()
            .filter(|p| panel.selected_paths.contains(*p))
            .cloned()
            .collect();
        if !ordered.is_empty() {
            return ordered;
        }
        let mut paths: Vec<PathBuf> = panel.selected_paths.iter().cloned().collect();
        paths.sort();
        return paths;
    }
    let targeted = panel.get_targeted_paths();
    if targeted.is_empty() {
        vec![panel.current_path.clone()]
    } else {
        targeted
    }
}

/// Join paths with newlines for a single clipboard payload.
pub fn format_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|p| Path::display(p).to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FileEntry;

    fn entry(name: &str, path: &str) -> FileEntry {
        FileEntry {
            name: name.into(),
            path: PathBuf::from(path),
            size: 0,
            is_dir: false,
            is_symlink: false,
            modified: None,
        }
    }

    #[test]
    fn format_paths_joins_with_newlines() {
        let paths = vec![PathBuf::from("a/b"), PathBuf::from("c")];
        assert_eq!(
            format_paths(&paths),
            format!(
                "{}\n{}",
                PathBuf::from("a/b").display(),
                PathBuf::from("c").display()
            )
        );
    }

    #[test]
    fn format_paths_empty_is_empty() {
        assert!(format_paths(&[]).is_empty());
    }

    #[test]
    fn paths_to_copy_uses_hovered_file() {
        let mut panel = PanelState::new(PathBuf::from("/tmp"));
        panel.entries = vec![
            entry("..", "/"),
            entry("a.rs", "/tmp/a.rs"),
            entry("b.rs", "/tmp/b.rs"),
        ];
        panel.cursor_index = 1;
        assert_eq!(paths_to_copy(&panel), vec![PathBuf::from("/tmp/a.rs")]);
    }

    #[test]
    fn paths_to_copy_falls_back_to_cwd_on_dotdot() {
        let mut panel = PanelState::new(PathBuf::from("/tmp"));
        panel.entries = vec![entry("..", "/")];
        panel.cursor_index = 0;
        assert_eq!(paths_to_copy(&panel), vec![PathBuf::from("/tmp")]);
    }

    #[test]
    fn paths_to_copy_keeps_selection_order() {
        let mut panel = PanelState::new(PathBuf::from("/tmp"));
        panel.entries = vec![
            entry("a.rs", "/tmp/a.rs"),
            entry("b.rs", "/tmp/b.rs"),
            entry("c.rs", "/tmp/c.rs"),
        ];
        panel.selected_paths.insert(PathBuf::from("/tmp/c.rs"));
        panel.selected_paths.insert(PathBuf::from("/tmp/a.rs"));
        panel.selection_order = vec![PathBuf::from("/tmp/c.rs"), PathBuf::from("/tmp/a.rs")];
        assert_eq!(
            paths_to_copy(&panel),
            vec![PathBuf::from("/tmp/c.rs"), PathBuf::from("/tmp/a.rs")]
        );
    }
}
