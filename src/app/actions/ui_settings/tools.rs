//! Tools, screens, and auxiliary dialog actions for UI settings.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::sys_helpers::get_process_list;
use crate::config::localization::t;
use crate::keybindings::Action;

pub fn handle_tools_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    match action {
        Action::EditUserMenu => {
            let path = crate::config::paths::get_config_dir().join("usermenu.toml");
            if !path.exists() {
                let default_template = r#"# Pairee User Custom Commands Menu
#
# Define your own custom commands here.
# Format:
# [commands]
# "Key" = "Command"
#
# Examples:
# "1" = "cargo build"
# "2" = "git status"
# "3" = "echo 'Hello World!'"
# "4" = "systemctl status docker"
"#;
                if let Err(e) = std::fs::write(&path, default_template) {
                    state.dialogs.replace(PopupType::Error(
                        t("error_read_usermenu_failed").replace("{}", &e.to_string()),
                    ));
                    return true;
                }
            }
            crate::app::editor::open::open_in_editor(state, path, &context.config.settings);
            true
        }
        Action::FileAssociations => {
            let config = crate::config::associations::AssociationsConfig::load();
            state.dialogs.replace(PopupType::FileAssociationsDialog {
                rules: config.rules,
                cursor_idx: 0,
                editing_idx: None,
                editing_field: 0,
                edit_buffer: String::new(),
                original_rule: None,
            });
            true
        }
        Action::FolderShortcutsConfig => {
            state
                .dialogs
                .replace(PopupType::FolderShortcuts { cursor_idx: 0 });
            true
        }
        Action::Hotlist => {
            let entries = crate::config::bookmarks::BookmarksFile::load().hotlist_entries();
            state.dialogs.replace(PopupType::Hotlist {
                entries,
                cursor_idx: 0,
            });
            true
        }
        Action::FilePanelFilter => {
            let active = state.get_active_panel();
            let current = active.filter_mask.clone().unwrap_or_default();
            state.dialogs.replace(PopupType::FilePanelFilterPrompt {
                input: current.into(),
            });
            true
        }
        Action::QuickFilter => {
            let active = state.get_active_panel();
            let current = active.quick_filter_mask.clone().unwrap_or_default();
            let original_mask = active.quick_filter_mask.clone();
            let original_cursor = active.cursor_index;
            state.dialogs.replace(PopupType::QuickFilterPrompt {
                input: current.into(),
                original_mask,
                original_cursor,
            });
            true
        }
        Action::TaskList => {
            let tasks = get_process_list();
            state.dialogs.replace(PopupType::TaskListDialog {
                tasks,
                cursor_idx: 0,
                filter_query: Default::default(),
                is_filtering: false,
            });
            true
        }
        Action::SaveSetup => {
            state.dialogs.replace(PopupType::SaveSetupConfirm);
            true
        }
        Action::SystemSettings => {
            state.dialogs.replace(PopupType::ConfigurationDialog(
                crate::app::state::ConfigurationDialogState::new(
                    &context.config.settings,
                    0,
                    0,
                    true,
                ),
            ));
            true
        }
        Action::FindFile => {
            let root = state.get_active_panel().current_path.clone();
            state.dialogs.replace(PopupType::SearchPrompt {
                query: String::new(),
                content_query: String::new(),
                search_root: root,
                case_sensitive: false,
                search_target: crate::fs::search::SearchTarget::Any,
                cursor_idx: 0,
            });
            true
        }
        Action::ScreensList => {
            let suspended = state.dialogs.take();
            state.dialogs.replace(PopupType::ScreensMenu {
                cursor_idx: state.active_screen_idx,
                suspended_popup: suspended.map(Box::new),
            });
            true
        }
        Action::NextScreen => {
            state.next_screen();
            true
        }
        Action::PrevScreen => {
            state.prev_screen();
            true
        }
        Action::VideoMode => {
            state.dialogs.replace(PopupType::Info(t("video_mode_hint")));
            true
        }
        Action::CycleFKeysModifiers => {
            use crossterm::event::KeyModifiers;
            state.fkeys_modifier_override = match state.fkeys_modifier_override {
                None => Some(KeyModifiers::CONTROL),
                Some(KeyModifiers::CONTROL) => Some(KeyModifiers::ALT),
                Some(KeyModifiers::ALT) => None,
                _ => None,
            };
            true
        }
        Action::CheckForUpdates => {
            if let Some(info) = state.update.available.clone() {
                state
                    .dialogs
                    .replace(crate::app::state::PopupType::UpdateAvailable {
                        info,
                        cursor_idx: 0,
                        install_progress: None,
                        error: None,
                        scroll_y: 0,
                    });
            } else {
                let cache = crate::config::paths::get_config_dir().join("update_cache.json");
                let _ = std::fs::remove_file(&cache);
                let (tx, rx) = tokio::sync::oneshot::channel();
                crate::update::checker::UpdateChecker::check_in_background(tx);
                state.update.check_rx = Some(rx);
                state.update.status = crate::update::UpdateStatus::Checking;
                state
                    .dialogs
                    .replace(crate::app::state::PopupType::Info(t("update_checking")));
            }
            true
        }
        Action::CommandPalette => {
            crate::app::actions::command_palette::open_palette(state);
            true
        }
        Action::WhichKey => {
            crate::app::actions::which_key::open_which_key(state, &context.resolver);
            true
        }
        Action::ToggleTransferPanel => {
            if let Some(ref mut ts) = state.transfer {
                match ts.view_mode {
                    crate::app::state::TransferViewMode::Hidden
                    | crate::app::state::TransferViewMode::Minimized => {
                        ts.view_mode = crate::app::state::TransferViewMode::Expanded;
                        state.dialogs.replace(PopupType::TransferPanel);
                    }
                    crate::app::state::TransferViewMode::Expanded => {
                        ts.view_mode = crate::app::state::TransferViewMode::Minimized;
                        state.dialogs.clear();
                    }
                }
            } else {
                state
                    .dialogs
                    .replace(PopupType::Info(t("transfer_no_active")));
            }
            true
        }
        _ => false,
    }
}
