mod git;
mod help;
mod plugins;
mod tools;
mod view_sort;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::app::sys_helpers::build_info_panel_lines;
use crate::config::localization::t;
use crate::keybindings::Action;

/// Handles UI, settings, and other configuration actions. Returns true if the action was handled.
pub async fn handle_ui_settings_action(
    state: &mut AppState,
    action: &Action,
    context: &mut AppContext,
) -> bool {
    // 1. Help & About
    match action {
        Action::About => {
            help::open_about(state);
            return true;
        }
        Action::Help => {
            help::open_help(state).await;
            return true;
        }
        _ => {}
    }

    // 2. Menus & general app controls
    match action {
        Action::UserMenu => {
            state.dialogs.replace(PopupType::UserMenu { cursor_idx: 0 });
            return true;
        }
        Action::Menu => {
            if let Some(PopupType::Menu { .. }) = state.dialogs.top() {
                state.dialogs.clear();
            } else {
                let active_item_idx = if context.config.settings.auto_drop_menu {
                    Some(0)
                } else {
                    None
                };
                state.dialogs.replace(PopupType::Menu {
                    active_menu_idx: 0,
                    active_item_idx,
                    active_submenu_idx: None,
                    active_submenu_item_idx: None,
                });
            }
            return true;
        }
        Action::ContextMenu => {
            let targets = state.get_active_panel().get_targeted_paths();
            if !targets.is_empty() {
                let mut items = vec![
                    t("ctx_menu_view"),
                    t("ctx_menu_edit"),
                    t("ctx_menu_copy"),
                    t("ctx_menu_move"),
                    t("ctx_menu_delete"),
                    t("ctx_menu_compress"),
                ];
                let has_archive = targets.iter().any(|p| {
                    let ext = p
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    matches!(
                        ext.as_str(),
                        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz"
                    )
                });
                if has_archive {
                    items.push(t("ctx_menu_extract"));
                }
                state.dialogs.replace(PopupType::ContextMenu {
                    items,
                    cursor_idx: 0,
                });
            }
            return true;
        }
        Action::Quit => {
            if context.config.settings.confirmations.confirm_quit {
                state.dialogs.replace(PopupType::ConfirmQuit);
            } else {
                state.should_quit = true;
            }
            return true;
        }
        Action::ToggleHidden => {
            context.config.settings.show_hidden = !context.config.settings.show_hidden;
            context.config.save_logging();
            state.refresh_both_panels(context.config.settings.show_hidden);
            return true;
        }
        Action::FocusCli => {
            state.cli_input.push(' ');
            state.cli_input.clear();
            return true;
        }
        Action::Unfocus => {
            state.dialogs.clear();
            state.cli_input.clear();
            state.fkeys_modifier_override = None;
            return true;
        }
        Action::Refresh | Action::RereadPanel => {
            state.refresh_both_panels(context.config.settings.show_hidden);
            return true;
        }
        Action::InfoPanel => {
            let lines = build_info_panel_lines(state);
            state.dialogs.replace(PopupType::InfoPanel { lines });
            return true;
        }
        Action::OpenGitPanel => return git::open_git_panel(state, context),
        Action::PluginMenu => {
            if !context.config.settings.plugins_enabled {
                state
                    .dialogs
                    .replace(PopupType::Info(t("feature_plugins_disabled")));
                return true;
            }
            plugins::open_plugin_menu(state, context);
            return true;
        }
        Action::InstallDevPlugin => return plugins::install_dev_plugin(state, context),
        _ => {}
    }

    // 3. View modes & sorting
    if view_sort::handle_view_sort_action(state, action, context) {
        return true;
    }

    // 4. Auxiliary tools, screens, dialogs
    tools::handle_tools_action(state, action, context)
}
