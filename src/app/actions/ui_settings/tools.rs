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
    config_action(state, action, context)
        || panel_prompt_action(state, action)
        || screen_action(state, action, context)
}

/// User menu, associations, shortcuts, hotlist and configuration dialogs.
fn config_action(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    match action {
        Action::EditUserMenu => edit_user_menu(state, context),
        Action::FileAssociations => {
            let config = crate::config::associations::AssociationsConfig::load();
            state.dialogs.replace(PopupType::FileAssociationsDialog {
                rules: config.rules,
                cursor_idx: 0,
                editing_idx: None,
                editing_field: 0,
                edit_buffer: Default::default(),
                original_rule: None,
            });
        }
        Action::FolderShortcutsConfig => {
            state
                .dialogs
                .replace(PopupType::FolderShortcuts { cursor_idx: 0 });
        }
        Action::Hotlist => {
            let entries = crate::config::bookmarks::BookmarksFile::load().hotlist_entries();
            state.dialogs.replace(PopupType::Hotlist {
                entries,
                cursor_idx: 0,
            });
        }
        Action::SaveSetup => state.dialogs.replace(PopupType::SaveSetupConfirm),
        Action::SystemSettings => {
            state.dialogs.replace(PopupType::ConfigurationDialog(
                crate::app::state::ConfigurationDialogState::new(&context.config, 0, 0, true),
            ));
        }
        _ => return false,
    }
    true
}

/// Filters, file search and the task list of the active panel.
fn panel_prompt_action(state: &mut AppState, action: &Action) -> bool {
    let active = state.get_active_panel();
    let popup = match action {
        Action::FilePanelFilter => PopupType::FilePanelFilterPrompt {
            input: active.filter_mask.clone().unwrap_or_default().into(),
        },
        Action::QuickFilter => PopupType::QuickFilterPrompt {
            input: active.quick_filter_mask.clone().unwrap_or_default().into(),
            original_mask: active.quick_filter_mask.clone(),
            original_cursor: active.cursor_index,
        },
        Action::FindFile => PopupType::SearchPrompt {
            query: Default::default(),
            content_query: Default::default(),
            search_root: active.current_path.clone(),
            case_sensitive: false,
            search_target: crate::fs::search::SearchTarget::Any,
            cursor_idx: 0,
        },
        Action::TaskList => PopupType::TaskListDialog {
            tasks: get_process_list(),
            cursor_idx: 0,
            filter_query: Default::default(),
            is_filtering: false,
        },
        _ => return false,
    };
    state.dialogs.replace(popup);
    true
}

/// Screens, key bar modifiers, palettes, updates and the transfer panel.
fn screen_action(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    match action {
        Action::ScreensList => {
            let suspended = state.dialogs.take();
            state.dialogs.replace(PopupType::ScreensMenu {
                cursor_idx: state.active_screen_idx,
                suspended_popup: suspended.map(Box::new),
            });
        }
        Action::NextScreen => state.next_screen(),
        Action::PrevScreen => state.prev_screen(),
        Action::VideoMode => state.dialogs.replace(PopupType::Info(t("video_mode_hint"))),
        Action::CycleFKeysModifiers => cycle_fkeys_modifiers(state),
        Action::CheckForUpdates => check_for_updates(state),
        Action::CommandPalette => crate::app::actions::command_palette::open_palette(state),
        Action::WhichKey => crate::app::shortcuts::open(state, context),
        Action::ToggleTransferPanel => toggle_transfer_panel(state),
        _ => return false,
    }
    true
}

/// Commented template written the first time the user menu is edited.
const USER_MENU_TEMPLATE: &str = r#"# Pairee User Custom Commands Menu
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

/// Opens `usermenu.toml` in the editor, creating it from the template.
fn edit_user_menu(state: &mut AppState, context: &AppContext) {
    let path = crate::config::paths::get_config_dir().join("usermenu.toml");
    if !path.exists()
        && let Err(e) = std::fs::write(&path, USER_MENU_TEMPLATE)
    {
        state.dialogs.replace(PopupType::Error(
            t("error_read_usermenu_failed").replace("{}", &e.to_string()),
        ));
        return;
    }
    crate::app::editor::open::open_in_editor(state, path, &context.config.settings);
}

/// Normal → Ctrl → Alt → Shift → Normal.
fn cycle_fkeys_modifiers(state: &mut AppState) {
    use crossterm::event::KeyModifiers as M;
    const CYCLE: [Option<M>; 4] = [None, Some(M::CONTROL), Some(M::ALT), Some(M::SHIFT)];
    let current = CYCLE
        .iter()
        .position(|m| *m == state.fkeys_modifier_override)
        .unwrap_or(CYCLE.len() - 1);
    state.fkeys_modifier_override = CYCLE[(current + 1) % CYCLE.len()];
}

/// Shows a known update, or starts a fresh check (dropping the cache).
fn check_for_updates(state: &mut AppState) {
    if let Some(info) = state.update.available.clone() {
        state.dialogs.replace(PopupType::UpdateAvailable {
            info,
            cursor_idx: 0,
            install_progress: None,
            error: None,
            scroll_y: 0,
        });
        return;
    }
    let cache = crate::config::paths::get_config_dir().join("update_cache.json");
    let _ = std::fs::remove_file(&cache);
    let (tx, rx) = tokio::sync::oneshot::channel();
    crate::update::checker::UpdateChecker::check_in_background(tx);
    state.update.check_rx = Some(rx);
    state.update.status = crate::update::UpdateStatus::Checking;
    state.dialogs.replace(PopupType::Info(t("update_checking")));
}

/// Expands a hidden/minimized transfer panel, or minimizes an expanded one.
fn toggle_transfer_panel(state: &mut AppState) {
    use crate::app::state::TransferViewMode;
    let Some(ref mut ts) = state.transfer else {
        state
            .dialogs
            .replace(PopupType::Info(t("transfer_no_active")));
        return;
    };
    match ts.view_mode {
        TransferViewMode::Hidden | TransferViewMode::Minimized => {
            ts.view_mode = TransferViewMode::Expanded;
            state.dialogs.replace(PopupType::TransferPanel);
        }
        TransferViewMode::Expanded => {
            ts.view_mode = TransferViewMode::Minimized;
            state.dialogs.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crossterm::event::KeyModifiers as M;

    #[test]
    fn fkey_modifier_cycle_includes_shift() {
        let mut state = AppState::new(".".into(), ".".into());
        let mut context = AppContext::new(AppConfig::default());
        let mut seen = Vec::new();
        for _ in 0..4 {
            handle_tools_action(&mut state, &Action::CycleFKeysModifiers, &mut context);
            seen.push(state.fkeys_modifier_override);
        }
        assert_eq!(seen, [Some(M::CONTROL), Some(M::ALT), Some(M::SHIFT), None]);
    }
}
