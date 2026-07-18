use crate::app::context::AppContext;
use crate::app::input::{handle_backspace_key, handle_enter_key};
use crate::app::state::{ActivePanel, AppState, PopupType, SelectMode};
use crate::app::sys_helpers::{build_tree_nodes, get_system_drives};
use crate::keybindings::Action;

/// Handles navigation, selection, and history actions. Returns `true` if the action was handled.
pub fn handle_navigation_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    match action {
        Action::MoveUp => {
            state.get_active_panel_mut().move_cursor_up();
            true
        }
        Action::MoveDown => {
            state.get_active_panel_mut().move_cursor_down();
            true
        }
        Action::PageUp => {
            state.get_active_panel_mut().page_up(10);
            true
        }
        Action::PageDown => {
            state.get_active_panel_mut().page_down(10);
            true
        }
        Action::GoToTop => {
            state.get_active_panel_mut().go_to_top();
            true
        }
        Action::GoToBottom => {
            state.get_active_panel_mut().go_to_bottom();
            true
        }
        Action::ChangePanel => {
            state.toggle_focus();
            true
        }
        Action::SelectItem => {
            let select_folders = state.select_folders;
            state
                .get_active_panel_mut()
                .toggle_selection_with_opts(select_folders);
            state.get_active_panel_mut().move_cursor_down();
            true
        }
        Action::Execute => {
            handle_enter_key(state, context);
            state.refresh_both_panels(context.config.settings.show_hidden);
            true
        }
        Action::GoParent => {
            handle_backspace_key(state, context.config.settings.show_hidden);
            true
        }
        Action::SwapPanels => {
            state.swap_panels();
            true
        }
        Action::DriveSelectLeft => {
            let drives = get_system_drives();
            state.active_popup = Some(PopupType::DriveSelect {
                panel: ActivePanel::Left,
                drives,
                cursor_idx: 0,
            });
            true
        }
        Action::DriveSelectRight => {
            let drives = get_system_drives();
            state.active_popup = Some(PopupType::DriveSelect {
                panel: ActivePanel::Right,
                drives,
                cursor_idx: 0,
            });
            true
        }
        Action::SshConnect => {
            let (name, host, port, user, pass, key_path, preset_idx, cursor_idx) =
                if !context.config.settings.ssh_presets.is_empty() {
                    let p = &context.config.settings.ssh_presets[0];
                    (
                        p.name.clone(),
                        p.host.clone(),
                        p.port.clone(),
                        p.username.clone(),
                        p.password.clone().unwrap_or_default(),
                        p.key_path.clone().unwrap_or_default(),
                        Some(0),
                        0,
                    )
                } else {
                    (
                        String::new(),
                        String::new(),
                        "22".to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        None,
                        1,
                    )
                };
            state.active_popup = Some(PopupType::SshConnectPrompt {
                panel: state.active_panel,
                input_name: name,
                input_host: host,
                input_port: port,
                input_user: user,
                input_pass: pass,
                input_key_path: key_path,
                cursor_idx,
                selected_preset_idx: preset_idx,
            });
            true
        }
        Action::SshDisconnect => {
            let panel = state.get_active_panel_mut();
            if panel.ssh_conn.is_some() {
                panel.ssh_conn = None;
                let local_dir =
                    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                panel.current_path = local_dir;
                panel.cursor_index = 0;
                panel.clear_selection();
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
            true
        }
        Action::GoFolderShortcut(n) => {
            if let Some(target) = state.folder_shortcuts.get(n).cloned() {
                let panel = state.get_active_panel_mut();
                panel.current_path = target;
                panel.cursor_index = 0;
                panel.clear_selection();
                state.refresh_both_panels(context.config.settings.show_hidden);
            } else {
                state.active_popup = Some(PopupType::Info(
                    crate::config::localization::t("error_no_folder_shortcut")
                        .replace("{}", &n.to_string()),
                ));
            }
            true
        }
        Action::SelectGroup => {
            state.active_popup = Some(PopupType::SelectGroupPrompt {
                mode: SelectMode::Add,
                query: String::new(),
            });
            true
        }
        Action::UnselectGroup => {
            state.active_popup = Some(PopupType::SelectGroupPrompt {
                mode: SelectMode::Remove,
                query: String::new(),
            });
            true
        }
        Action::InvertSelection => {
            state.snapshot_selection();
            state.get_active_panel_mut().invert_selection();
            true
        }
        Action::RestoreSelection => {
            state.restore_selection();
            true
        }
        Action::TreeView => {
            let root = state.get_active_panel().current_path.clone();
            let nodes = build_tree_nodes(&root, 0, 3);
            state.active_popup = Some(PopupType::TreeView {
                nodes,
                cursor_idx: 0,
                caller: crate::app::state::types::TreeViewCaller::Panel(state.active_panel),
            });
            true
        }
        Action::CommandHistory => {
            let entries = state.command_history.clone();
            state.active_popup = Some(PopupType::CommandHistoryList {
                entries,
                cursor_idx: 0,
            });
            true
        }
        Action::FileViewHistory => {
            let entries = state.file_view_history.clone();
            state.active_popup = Some(PopupType::FileViewHistoryList {
                entries,
                cursor_idx: 0,
            });
            true
        }
        Action::FoldersHistory => {
            let entries = state.folders_history.clone();
            state.active_popup = Some(PopupType::FoldersHistoryList {
                entries,
                cursor_idx: 0,
            });
            true
        }
        _ => false,
    }
}
