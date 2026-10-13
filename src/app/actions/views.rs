//! Keymap actions of the views over the panels' files: branch view, folder
//! jump, Compare files and the duplicate finder.

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::state::popup::DupeOptionsPrompt;
use crate::app::text_input::TextField;
use crate::fs::dupes::DupeOptions;
use crate::keybindings::Action;

pub fn handle(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    match action {
        Action::BranchView => toggle_branch_view(state, context.config.settings.show_hidden),
        Action::FolderJump => crate::app::folder_jump::open(state),
        Action::CompareFiles => crate::app::file_compare::open(state),
        Action::FindDuplicates => open_dupe_options(state),
        _ => false,
    }
}

/// Turns the branch view of the active panel on or off and rereads it.
fn toggle_branch_view(state: &mut AppState, show_hidden: bool) -> bool {
    let panel = state.get_active_panel_mut();
    panel.branch_view = !panel.branch_view;
    panel.cursor_index = 0;
    panel.clear_selection();
    state.refresh_panel(state.panels.active, show_hidden, true);
    true
}

/// The duplicate finder options, searching the selection (or the current
/// folder when nothing is selected).
fn open_dupe_options(state: &mut AppState) -> bool {
    let panel = state.get_active_panel();
    let roots = if panel.selection_order.is_empty() {
        vec![panel.current_path.clone()]
    } else {
        panel.selection_order.clone()
    };
    let defaults = DupeOptions::default();
    state.dialogs.replace(DupeOptionsPrompt {
        roots,
        min_size: TextField::new(defaults.min_size.to_string().as_str()),
        algorithm: defaults.algorithm,
        cursor_idx: 0,
    });
    true
}
