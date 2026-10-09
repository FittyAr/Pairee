//! Copy paths or names of the targeted items to the system clipboard
//! (`copy_path`, `copy_name`, `copy_name_no_ext`, `copy_dir_path`).

use crate::app::state::{AppState, PanelState, PopupType};
use crate::app::sys_helpers::clipboard;
use crate::config::localization::t;
use std::path::{Path, PathBuf};

/// What goes on the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathText {
    /// Full paths of the targeted items.
    FullPath,
    /// Their file names.
    Name,
    /// Their file names without extension.
    NameNoExt,
    /// The panel's folder.
    Folder,
}

pub fn handle(state: &mut AppState, what: PathText) -> bool {
    let text = clipboard::format_paths(&texts(state.get_active_panel(), what));
    match clipboard::set_text(&text) {
        Ok(()) => {
            state
                .dialogs
                .replace(PopupType::Info(t("clipboard_copied").replace("{}", &text)));
        }
        Err(e) => {
            state
                .dialogs
                .replace(PopupType::Error(t("clipboard_failed").replace("{}", &e)));
        }
    }
    true
}

fn texts(panel: &PanelState, what: PathText) -> Vec<PathBuf> {
    let part = |p: &PathBuf, f: fn(&Path) -> Option<&std::ffi::OsStr>| {
        f(p).map_or_else(|| p.clone(), PathBuf::from)
    };
    match what {
        PathText::Folder => vec![panel.current_path.clone()],
        PathText::FullPath => clipboard::paths_to_copy(panel),
        PathText::Name => clipboard::paths_to_copy(panel)
            .iter()
            .map(|p| part(p, Path::file_name))
            .collect(),
        PathText::NameNoExt => clipboard::paths_to_copy(panel)
            .iter()
            .map(|p| part(p, Path::file_stem))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_stems_of_the_targets() {
        let mut panel = PanelState::new(PathBuf::from("/d"));
        for name in ["report.pdf", "notes.tar.gz"] {
            let path = PathBuf::from("/d").join(name);
            panel.selected_paths.insert(path.clone());
            panel.selection_order.push(path);
        }
        let render = |what| clipboard::format_paths(&texts(&panel, what));
        assert_eq!(render(PathText::Name), "report.pdf\nnotes.tar.gz");
        assert_eq!(render(PathText::NameNoExt), "report\nnotes.tar");
        assert_eq!(
            render(PathText::Folder),
            PathBuf::from("/d").display().to_string()
        );
    }
}
