use super::super::DEV_OPT_COUNT;
use super::super::progress::dev_op_running;
use crate::app::context::AppContext;
use crate::app::state::{AppState, PluginMenuState};
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle_navigation_or_enter(
    key: KeyEvent,
    state: &mut AppState,
    context: &mut AppContext,
    menu: &mut PluginMenuState,
    panels: (&std::path::Path, &std::path::Path),
) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
            let has_active = context.config.settings.active_dev_plugin.is_some();
            if menu.cursor_idx == 0 {
                menu.cursor_idx = DEV_OPT_COUNT - 1;
            } else if has_active && menu.cursor_idx == 2 {
                menu.cursor_idx = 0;
            } else {
                menu.cursor_idx -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
            let has_active = context.config.settings.active_dev_plugin.is_some();
            if menu.cursor_idx >= DEV_OPT_COUNT - 1 {
                menu.cursor_idx = 0;
            } else if has_active && menu.cursor_idx == 0 {
                menu.cursor_idx = 2;
            } else {
                menu.cursor_idx += 1;
            }
        }
        KeyCode::Backspace | KeyCode::Delete | KeyCode::Char('d') | KeyCode::Char('D') => {
            if menu.cursor_idx == 0 && context.config.settings.active_dev_plugin.is_some() {
                context.config.settings.active_dev_plugin = None;
                context.config.save_logging();
                menu.dev_results = t("plugin_dev_deselected");
                menu.installed = super::super::reload_installed_plugins(context, &None);
            }
        }
        KeyCode::Enter => {
            let plugins_dev_dir =
                std::path::PathBuf::from(context.config.settings.plugins_dev_dir.clone());
            let active_plugin = context.config.settings.active_dev_plugin.clone();

            if dev_op_running(state) {
                menu.dev_results = t("plugin_dev_op_in_progress");
                return;
            }

            match menu.cursor_idx {
                0 => super::super::actions::handle_option_select_active_plugin(
                    context,
                    menu,
                    panels,
                    plugins_dev_dir,
                ),
                1 => super::super::actions::handle_option_init_plugin(active_plugin, menu),
                2 => super::super::actions::handle_option_lint(
                    state,
                    context,
                    &mut menu.dev_results,
                    active_plugin,
                    plugins_dev_dir,
                ),
                3 => super::super::actions::handle_option_package(
                    state,
                    context,
                    &mut menu.dev_results,
                    active_plugin,
                    plugins_dev_dir,
                ),
                4 => super::super::actions::handle_option_install_local(
                    state,
                    context,
                    &mut menu.dev_results,
                    active_plugin,
                    plugins_dev_dir,
                ),
                5 => super::super::actions::handle_option_submit(
                    menu,
                    active_plugin,
                    plugins_dev_dir,
                ),
                6 => super::super::actions::handle_option_open_dev_folder(
                    state,
                    context,
                    &mut menu.dev_results,
                ),
                7 => super::super::actions::handle_option_open_package_folder(
                    state,
                    context,
                    &mut menu.dev_results,
                    active_plugin,
                ),
                8 => super::super::actions::handle_option_open_submit_folder(
                    state,
                    context,
                    &mut menu.dev_results,
                ),
                _ => {}
            }
        }
        _ => {}
    }
}
