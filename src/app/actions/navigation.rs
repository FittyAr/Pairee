use crate::app::context::AppContext;
use crate::app::input::{handle_backspace_key, handle_enter_key, handle_open_archive_key};
use crate::app::state::history::HistoryKind;
use crate::app::state::popup::DriveSelect;
use crate::app::state::popup::SelectGroupPrompt;
use crate::app::state::popup::TreeView;
use crate::app::state::{ActivePanel, AppState, PopupType, SelectMode};
use crate::app::sys_helpers::{build_tree_nodes, get_system_drives};
use crate::config::localization::t;
use crate::keybindings::Action;

/// Rows a page step moves the cursor.
const PAGE_ROWS: usize = 10;

/// Handles navigation, selection, and history actions. Returns `true` if the action was handled.
pub fn handle_navigation_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    cursor_action(state, action)
        || panel_action(state, action, context)
        || selection_action(state, action)
        || list_dialog_action(state, action)
}

/// Cursor movement inside the active panel and focus switching.
fn cursor_action(state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::ChangePanel => state.toggle_focus(),
        Action::SwapPanels => state.swap_panels(),
        Action::FocusLeftPanel => state.panels.active = ActivePanel::Left,
        Action::FocusRightPanel => state.panels.active = ActivePanel::Right,
        _ => {
            let panel = state.get_active_panel_mut();
            match action {
                Action::MoveUp => panel.move_cursor_up(),
                Action::MoveDown => panel.move_cursor_down(),
                Action::PageUp => panel.page_up(PAGE_ROWS),
                Action::PageDown => panel.page_down(PAGE_ROWS),
                Action::HalfPageUp => panel.page_up(PAGE_ROWS / 2),
                Action::HalfPageDown => panel.page_down(PAGE_ROWS / 2),
                Action::GoToTop => panel.go_to_top(),
                Action::GoToBottom => panel.go_to_bottom(),
                _ => return false,
            }
            panel.extend_visual();
        }
    }
    true
}

/// Changing folder, drive or connection of the active panel.
fn panel_action(state: &mut AppState, action: &Action, context: &mut AppContext) -> bool {
    let show_hidden = context.config.settings.show_hidden;
    match action {
        Action::Execute => {
            // Only the focused panel can change directory on Enter.
            handle_enter_key(state, context);
            state.refresh_active_panel(show_hidden);
        }
        Action::OpenArchive => {
            handle_open_archive_key(state, context);
            state.refresh_active_panel(show_hidden);
        }
        Action::GoParent => handle_backspace_key(state, show_hidden),
        Action::DriveSelectLeft => open_drive_select(state, ActivePanel::Left),
        Action::DriveSelectRight => open_drive_select(state, ActivePanel::Right),
        Action::SshConnect => open_ssh_connect(state, context),
        Action::SshDisconnect => ssh_disconnect(state, show_hidden),
        Action::GoFolderShortcut(n) => go_folder_shortcut(state, *n, show_hidden),
        _ => return false,
    }
    true
}

/// Selection of panel entries.
fn selection_action(state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::SelectItem => select_item(state),
        Action::SelectGroup => open_select_group(state, SelectMode::Add),
        Action::UnselectGroup => open_select_group(state, SelectMode::Remove),
        Action::InvertSelection => {
            state.snapshot_selection();
            state.get_active_panel_mut().invert_selection();
        }
        Action::RestoreSelection => state.restore_selection(),
        Action::SelectAll => {
            state.snapshot_selection();
            state.get_active_panel_mut().select_group("*");
        }
        Action::UnselectAll => {
            state.snapshot_selection();
            state.get_active_panel_mut().clear_selection();
        }
        Action::VisualMode => state.get_active_panel_mut().toggle_visual(),
        _ => return false,
    }
    true
}

/// Tree view and history lists.
fn list_dialog_action(state: &mut AppState, action: &Action) -> bool {
    let popup = match action {
        Action::TreeView => {
            let root = state.get_active_panel().current_path.clone();
            PopupType::TreeView(TreeView {
                nodes: build_tree_nodes(&root, 0, 3),
                cursor_idx: 0,
                caller: crate::app::state::types::TreeViewCaller::Panel(state.panels.active),
            })
        }
        Action::CommandHistory => HistoryKind::Commands.list_popup(&state.history, 0),
        Action::FileViewHistory => HistoryKind::ViewedFiles.list_popup(&state.history, 0),
        Action::FoldersHistory => HistoryKind::Folders.list_popup(&state.history, 0),
        _ => return false,
    };
    state.dialogs.replace(popup);
    true
}

fn select_item(state: &mut AppState) {
    let panel = state.get_active_panel_mut();
    let cursor = panel.cursor_index;
    let selected = !panel.is_selected(cursor);
    set_selected(state, cursor, selected);
    state.get_active_panel_mut().move_cursor_down();
}

/// Tags or untags entry `idx` of the active panel. Like Far/MC, tagging a
/// folder also measures it.
pub(crate) fn set_selected(state: &mut AppState, idx: usize, selected: bool) {
    let select_folders = state.prefs.select_folders;
    let panel = state.get_active_panel_mut();
    if selected && let Some(entry) = panel.entries.get(idx) {
        let path = entry.path.clone();
        panel.calculate_dir_sizes(&[path]);
    }
    panel.set_selected(idx, selected, select_folders);
}

fn open_select_group(state: &mut AppState, mode: SelectMode) {
    state.dialogs.replace(SelectGroupPrompt {
        mode,
        query: Default::default(),
    });
}

fn open_drive_select(state: &mut AppState, panel: ActivePanel) {
    state.dialogs.replace(DriveSelect {
        panel,
        drives: get_system_drives(),
        cursor_idx: 0,
    });
}

fn open_ssh_connect(state: &mut AppState, context: &AppContext) {
    if !context.config.settings.ssh_enabled {
        state
            .dialogs
            .replace(PopupType::info(t("feature_ssh_disabled")));
        return;
    }
    state.dialogs.replace(PopupType::ConnectionPrompt(
        crate::app::state::ConnectionPromptState::new(
            state.panels.active,
            &context.config.settings.connections,
        ),
    ));
}

/// Returns a remote panel to the local working directory.
fn ssh_disconnect(state: &mut AppState, show_hidden: bool) {
    let panel = state.get_active_panel_mut();
    if panel.source.remote().is_none() {
        return;
    }
    panel.source = crate::fs::vfs::PanelSource::Local;
    panel.current_path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    panel.cursor_index = 0;
    panel.clear_selection();
    state.refresh_both_panels(show_hidden);
}

fn go_folder_shortcut(state: &mut AppState, n: u8, show_hidden: bool) {
    if let Some(target) = state.folder_shortcuts.get(&n).cloned() {
        state.jump_active_panel_to(target, show_hidden);
    } else {
        state.dialogs.replace(PopupType::info(
            t("error_no_folder_shortcut").replace("{}", &n.to_string()),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::InfoMessage;
    use crate::config::AppConfig;
    use std::path::PathBuf;

    fn ctx() -> AppContext {
        AppContext::new(AppConfig::default())
    }

    #[test]
    fn ssh_connect_disabled_shows_info() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        let mut context = ctx();
        context.config.settings.ssh_enabled = false;
        assert!(handle_navigation_action(
            &mut state,
            &Action::SshConnect,
            &mut context
        ));
        match state.dialogs.top() {
            Some(PopupType::Info(InfoMessage(msg))) => assert!(!msg.is_empty()),
            other => panic!("expected Info, got {other:?}"),
        }
    }
}
