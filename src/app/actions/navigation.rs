use crate::app::context::AppContext;
use crate::app::input::{handle_backspace_key, handle_enter_key, handle_open_archive_key};
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
            PopupType::TreeView {
                nodes: build_tree_nodes(&root, 0, 3),
                cursor_idx: 0,
                caller: crate::app::state::types::TreeViewCaller::Panel(state.panels.active),
            }
        }
        Action::CommandHistory => PopupType::CommandHistoryList {
            entries: state.history.commands.clone(),
            cursor_idx: 0,
        },
        Action::FileViewHistory => PopupType::FileViewHistoryList {
            entries: state.history.viewed_files.clone(),
            cursor_idx: 0,
        },
        Action::FoldersHistory => PopupType::FoldersHistoryList {
            entries: state.history.folders.clone(),
            cursor_idx: 0,
        },
        _ => return false,
    };
    state.dialogs.replace(popup);
    true
}

fn select_item(state: &mut AppState) {
    let select_folders = state.select_folders;
    let panel = state.get_active_panel_mut();
    // Like Far/MC: selecting a folder also measures it.
    if let Some(entry) = panel.entries.get(panel.cursor_index) {
        let path = entry.path.clone();
        panel.calculate_dir_sizes(&[path]);
    }
    panel.toggle_selection_with_opts(select_folders);
    panel.move_cursor_down();
}

fn open_select_group(state: &mut AppState, mode: SelectMode) {
    state.dialogs.replace(PopupType::SelectGroupPrompt {
        mode,
        query: Default::default(),
    });
}

fn open_drive_select(state: &mut AppState, panel: ActivePanel) {
    state.dialogs.replace(PopupType::DriveSelect {
        panel,
        drives: get_system_drives(),
        cursor_idx: 0,
    });
}

fn open_ssh_connect(state: &mut AppState, context: &AppContext) {
    if !context.config.settings.ssh_enabled {
        state
            .dialogs
            .replace(PopupType::Info(t("feature_ssh_disabled")));
        return;
    }
    state.dialogs.replace(PopupType::SshConnectPrompt(
        crate::app::state::SshConnectPromptState::new(
            state.panels.active,
            &context.config.settings.ssh_presets,
        ),
    ));
}

/// Returns an SSH panel to the local working directory.
fn ssh_disconnect(state: &mut AppState, show_hidden: bool) {
    let panel = state.get_active_panel_mut();
    if panel.source.ssh().is_none() {
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
        state.dialogs.replace(PopupType::Info(
            t("error_no_folder_shortcut").replace("{}", &n.to_string()),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            Some(PopupType::Info(msg)) => assert!(!msg.is_empty()),
            other => panic!("expected Info, got {other:?}"),
        }
    }
}
