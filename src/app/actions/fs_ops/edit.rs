use crate::app::context::AppContext;
use crate::app::editor::open::open_in_editor;
use crate::app::editor::remote::open_remote;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::config::settings::Settings;
use std::path::PathBuf;

/// F4: opens the file under the cursor in the built-in editor.
pub fn handle(state: &mut AppState, context: &mut AppContext) -> bool {
    let active = state.get_active_panel();
    if let Some(entry) = active
        .entries
        .get(active.cursor_index)
        .filter(|e| !e.is_dir)
    {
        let path = entry.path.clone();
        edit_file(state, path, &context.config.settings);
    }
    true
}

/// Opens `path` in the editor. An entry of a non-local active panel (SFTP
/// server, zip archive) is edited through a local copy uploaded on save.
pub fn edit_file(state: &mut AppState, path: PathBuf, settings: &Settings) {
    let panel = state.get_active_panel();
    let listed_remote = !panel.source.is_local() && panel.entries.iter().any(|e| e.path == path);
    if !listed_remote {
        state.push_file_view_history(path.clone());
        open_in_editor(state, path, settings);
        return;
    }
    let vfs = panel.source.vfs_for(&path);
    if vfs.capabilities().write {
        open_remote(state, vfs, path);
    } else {
        state
            .dialogs
            .replace(PopupType::Info(t("vfs_action_unsupported")));
    }
}
