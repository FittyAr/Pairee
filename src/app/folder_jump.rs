//! Folder jump (zoxide-like): the visited folders, best frecency first,
//! filtered by a fuzzy query as it is typed. Enter goes there.

use crate::app::input::cli_cd::go_to_local;
use crate::app::state::popup::FolderJump;
use crate::app::state::{AppState, PopupType};
use crate::app::text_input::TextField;
use crate::config::frecency::now;
use crate::config::localization::t;
use std::path::PathBuf;

/// Opens the folder jump list, without the local folders that no longer
/// exist (forgotten). Folders on network shares are kept unchecked: an
/// unreachable server must not stall the list, and may come back.
pub fn open(state: &mut AppState) -> bool {
    state
        .frecency
        .prune(|path| !crate::fs::watch::needs_polling(path) && !path.is_dir());
    let items = matches(state, "");
    state.dialogs.replace(FolderJump {
        query: TextField::default(),
        cursor_idx: 0,
        items,
    });
    true
}

/// The visited folders matching `query`, best first.
pub fn matches(state: &AppState, query: &str) -> Vec<PathBuf> {
    state
        .frecency
        .ranked(query, now())
        .into_iter()
        .map(|found| found.path)
        .collect()
}

/// Goes to `path` on the active panel; a folder that no longer exists is
/// forgotten and reported. The caller rereads the panels.
pub fn go(state: &mut AppState, path: PathBuf) {
    state.dialogs.clear();
    if !path.is_dir() {
        state.frecency.remove(&path);
        let text = t("folder_jump_missing").replacen("{}", &path.to_string_lossy(), 1);
        state.dialogs.replace(PopupType::error(text));
        return;
    }
    let current = state.get_active_panel().current_path.clone();
    go_to_local(state, current, path);
}
