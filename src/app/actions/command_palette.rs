//! Command palette (Ctrl+Shift+P): fuzzy list of logical actions.

use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crate::keybindings::preset::parse_action_name;

/// Build the full catalogue of palette entries (label, action).
pub fn all_palette_items() -> Vec<(String, Action)> {
    // Keep labels human-readable; filter matches against both label and key.
    let names = [
        "move_up",
        "move_down",
        "change_panel",
        "help",
        "about",
        "copy",
        "move",
        "rename",
        "delete",
        "mkdir",
        "view",
        "edit",
        "find_file",
        "refresh",
        "toggle_hidden",
        "swap_panels",
        "open_git_panel",
        "ssh_connect",
        "ssh_disconnect",
        "plugin_menu",
        "system_settings",
        "check_for_updates",
        "toggle_transfer_panel",
        "quit",
        "compare_folder",
        "task_list",
        "tree_view",
        "command_history",
        "folders_history",
        "file_view_history",
        "save_setup",
        "user_menu",
        "file_associations",
        "compress_files",
        "extract_archive",
    ];

    let mut items = Vec::with_capacity(names.len());
    for name in names {
        if let Some(action) = parse_action_name(name) {
            let label = name.replace('_', " ");
            items.push((label, action));
        }
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items
}

pub fn filter_items(query: &str) -> Vec<(String, Action)> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return all_palette_items();
    }
    all_palette_items()
        .into_iter()
        .filter(|(label, _)| {
            label.to_lowercase().contains(&q) || label.replace(' ', "_").contains(&q)
        })
        .collect()
}

/// Open the palette popup on `state`.
pub fn open_palette(state: &mut AppState) {
    let items = all_palette_items();
    state.active_popup = Some(PopupType::CommandPalette {
        query: String::new(),
        cursor_idx: 0,
        items,
    });
}
