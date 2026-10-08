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

    // 2. Menus & general app controls, 3. view modes & sorting,
    // 4. auxiliary tools, screens, dialogs
    menu_action(state, action, context)
        || app_control_action(state, action, context)
        || feature_action(state, action, context)
        || view_sort::handle_view_sort_action(state, action, context)
        || tools::handle_tools_action(state, action, context)
}

/// Menus, the context menu and quitting.
fn menu_action(state: &mut AppState, action: &Action, context: &AppContext) -> bool {
    match action {
        Action::UserMenu => state.dialogs.replace(PopupType::UserMenu { cursor_idx: 0 }),
        Action::Menu => toggle_main_menu(state, context),
        Action::ContextMenu => open_context_menu(state),
        Action::Quit => {
            if context.config.settings.confirmations.confirm_quit {
                state.dialogs.replace(PopupType::ConfirmQuit);
            } else {
                state.should_quit = true;
            }
        }
        _ => return false,
    }
    true
}

/// Hidden files, command line focus, Esc, refresh and the info panel.
fn app_control_action(state: &mut AppState, action: &Action, context: &mut AppContext) -> bool {
    match action {
        Action::ToggleHidden => {
            context.config.settings.show_hidden = !context.config.settings.show_hidden;
            context.config.save_logging();
            state.refresh_both_panels(context.config.settings.show_hidden);
        }
        Action::FocusCli => {
            state.cli_input.push(' ');
            state.cli_input.clear();
        }
        Action::Unfocus => unfocus(state),
        Action::Refresh | Action::RereadPanel => {
            state.force_refresh_both_panels(context.config.settings.show_hidden);
        }
        Action::InfoPanel => {
            let lines = build_info_panel_lines(state);
            state.dialogs.replace(PopupType::InfoPanel { lines });
        }
        _ => return false,
    }
    true
}

/// Git and plugin entry points.
fn feature_action(state: &mut AppState, action: &Action, context: &mut AppContext) -> bool {
    match action {
        Action::OpenGitPanel => git::open_git_panel(state, context),
        Action::GitInit => git::init_repo_action(state, context),
        Action::GitClone => git::clone_repo_prompt_action(state, context),
        Action::PluginMenu => {
            if context.config.settings.plugins_enabled {
                plugins::open_plugin_menu(state, context);
            } else {
                state
                    .dialogs
                    .replace(PopupType::Info(t("feature_plugins_disabled")));
            }
            true
        }
        Action::InstallDevPlugin => plugins::install_dev_plugin(state, context),
        _ => false,
    }
}

fn toggle_main_menu(state: &mut AppState, context: &AppContext) {
    if let Some(PopupType::Menu { .. }) = state.dialogs.top() {
        state.dialogs.clear();
        return;
    }
    let active_item_idx = context.config.settings.auto_drop_menu.then_some(0);
    state.dialogs.replace(PopupType::Menu {
        active_menu_idx: 0,
        active_item_idx,
        active_submenu_idx: None,
        active_submenu_item_idx: None,
    });
}

/// Extensions that add "Extract" to the context menu.
const ARCHIVE_EXTENSIONS: [&str; 7] = ["zip", "7z", "rar", "tar", "gz", "bz2", "xz"];

fn open_context_menu(state: &mut AppState) {
    let targets = state.get_active_panel().get_targeted_paths();
    if targets.is_empty() {
        return;
    }
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
        ARCHIVE_EXTENSIONS.contains(&ext.as_str())
    });
    if has_archive {
        items.push(t("ctx_menu_extract"));
    }
    state.dialogs.replace(PopupType::ContextMenu {
        items,
        cursor_idx: 0,
    });
}

/// Esc: closes dialogs, clears the command line and stops folder size calculations.
fn unfocus(state: &mut AppState) {
    state.dialogs.clear();
    state.cli_input.clear();
    state.fkeys_modifier_override = None;
    for (_, tab) in state.panels.all_tabs_mut() {
        tab.panel.dir_sizes.cancel();
    }
}
