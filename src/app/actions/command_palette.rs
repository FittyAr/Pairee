//! Command palette (Ctrl+Shift+P): fuzzy list of logical actions.

use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crate::keybindings::preset::parse_action_name;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

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
        "copy_path",
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
    let items = all_palette_items();
    let q = query.trim();
    if q.is_empty() {
        return items;
    }
    let mut matcher = Matcher::new(Config::DEFAULT);
    let pattern = Pattern::parse(q, CaseMatching::Ignore, Normalization::Smart);
    let mut buf = Vec::new();
    let mut scored: Vec<(u32, String, Action)> = Vec::new();
    for (label, action) in items {
        let hay = format!("{label} {}", label.replace(' ', "_"));
        let utf = Utf32Str::new(&hay, &mut buf);
        if let Some(score) = pattern.score(utf, &mut matcher) {
            scored.push((score, label, action));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    scored
        .into_iter()
        .map(|(_, label, action)| (label, action))
        .collect()
}

/// Open the palette popup on `state`.
pub fn open_palette(state: &mut AppState) {
    let items = all_palette_items();
    state.dialogs.replace(PopupType::CommandPalette {
        query: String::new(),
        cursor_idx: 0,
        items,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_includes_copy_path() {
        assert!(
            all_palette_items()
                .iter()
                .any(|(_, action)| *action == Action::CopyPath)
        );
    }

    #[test]
    fn filter_copy_path_matches_query() {
        let items = filter_items("copy path");
        assert!(items.iter().any(|(_, action)| *action == Action::CopyPath));
    }

    #[test]
    fn filter_fuzzy_abbreviation_finds_copy_path() {
        let items = filter_items("cpth");
        assert!(
            items.iter().any(|(_, action)| *action == Action::CopyPath),
            "nucleo should fuzzy-match 'cpth' to copy path, got {items:?}"
        );
    }

    #[test]
    fn filter_ranks_better_matches_first() {
        let items = filter_items("copy");
        let pos_copy = items.iter().position(|(_, a)| *a == Action::Copy);
        let pos_path = items.iter().position(|(_, a)| *a == Action::CopyPath);
        assert!(pos_copy.is_some() && pos_path.is_some());
        assert!(
            pos_copy.unwrap() <= pos_path.unwrap(),
            "exact 'copy' should rank at least as high as 'copy path', got {items:?}"
        );
    }

    #[test]
    fn filter_unknown_query_is_empty() {
        assert!(filter_items("zzzz-no-such-action").is_empty());
    }
}
