use super::super::DEV_OPT_COUNT;
use super::super::progress::dev_op_running;
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

#[allow(clippy::too_many_arguments)]
pub fn handle_navigation_or_enter(
    key: KeyEvent,
    state: &mut AppState,
    context: &mut AppContext,
    cursor_idx: &mut usize,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    search_query: &mut String,
    editing_query: &mut bool,
    dev_results: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
    left_panel_path: &std::path::Path,
    right_panel_path: &std::path::Path,
) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            let has_active = context.config.settings.active_dev_plugin.is_some();
            if *cursor_idx == 0 {
                *cursor_idx = DEV_OPT_COUNT - 1;
            } else if has_active && *cursor_idx == 2 {
                *cursor_idx = 0;
            } else {
                *cursor_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            let has_active = context.config.settings.active_dev_plugin.is_some();
            if *cursor_idx >= DEV_OPT_COUNT - 1 {
                *cursor_idx = 0;
            } else if has_active && *cursor_idx == 0 {
                *cursor_idx = 2;
            } else {
                *cursor_idx += 1;
            }
        }
        KeyCode::Backspace | KeyCode::Delete | KeyCode::Char('d') | KeyCode::Char('D') => {
            if *cursor_idx == 0 && context.config.settings.active_dev_plugin.is_some() {
                context.config.settings.active_dev_plugin = None;
                context.config.save_logging();
                *dev_results = t("plugin_dev_deselected");
                *installed = super::super::reload_installed_plugins(context, &None);
            }
        }
        KeyCode::Enter => {
            let plugins_dev_dir =
                std::path::PathBuf::from(context.config.settings.plugins_dev_dir.clone());
            let active_plugin = context.config.settings.active_dev_plugin.clone();

            if dev_op_running(state) {
                *dev_results = t("plugin_dev_op_in_progress");
                return;
            }

            match *cursor_idx {
                0 => super::super::actions::handle_option_select_active_plugin(
                    context,
                    dev_results,
                    installed,
                    left_panel_path,
                    right_panel_path,
                    plugins_dev_dir,
                ),
                1 => super::super::actions::handle_option_init_plugin(
                    context,
                    active_plugin,
                    editing_query,
                    search_query,
                    dev_results,
                    dev_wizard_step,
                    dev_wizard_data,
                ),
                2 => super::super::actions::handle_option_lint(
                    state,
                    context,
                    dev_results,
                    active_plugin,
                    plugins_dev_dir,
                ),
                3 => super::super::actions::handle_option_package(
                    state,
                    context,
                    dev_results,
                    active_plugin,
                    plugins_dev_dir,
                ),
                4 => super::super::actions::handle_option_install_local(
                    state,
                    context,
                    dev_results,
                    active_plugin,
                    plugins_dev_dir,
                ),
                5 => super::super::actions::handle_option_submit(
                    context,
                    dev_results,
                    active_plugin,
                    plugins_dev_dir,
                    editing_query,
                    search_query,
                    dev_wizard_step,
                    dev_wizard_data,
                ),
                6 => super::super::actions::handle_option_open_dev_folder(
                    state,
                    context,
                    dev_results,
                ),
                7 => super::super::actions::handle_option_open_package_folder(
                    state,
                    context,
                    dev_results,
                    active_plugin,
                ),
                8 => super::super::actions::handle_option_open_submit_folder(
                    state,
                    context,
                    dev_results,
                ),
                _ => {}
            }
        }
        _ => {}
    }
}
