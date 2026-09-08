//! Developer Tools key-event handler.

mod nav;
mod wizard;

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::localization::t;
use crossterm::event::{KeyCode, KeyEvent};

#[allow(clippy::too_many_arguments)]
pub fn handle_dev(
    key: KeyEvent,
    state: &mut AppState,
    context: &mut AppContext,
    left_panel_path: &std::path::Path,
    right_panel_path: &std::path::Path,
    cursor_idx: &mut usize,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    search_query: &mut String,
    editing_query: &mut bool,
    dev_results: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
) {
    let active_plugin = context.config.settings.active_dev_plugin.clone();
    if let Some(ref folder_name) = active_plugin {
        let plugins_dev_dir = &context.config.settings.plugins_dev_dir;
        let path = if std::path::Path::new(folder_name).is_absolute() {
            std::path::PathBuf::from(folder_name)
        } else {
            std::path::PathBuf::from(plugins_dev_dir).join(folder_name)
        };
        if !path.exists() || !path.is_dir() || !path.join("manifest.toml").exists() {
            context.config.settings.active_dev_plugin = None;
            context.config.save_logging();
            *dev_results = t("plugin_dev_stale_deselected");
            *installed = super::reload_installed_plugins(context, &None);
        }
    }

    if *editing_query {
        handle_editing_query(
            key,
            state,
            context,
            cursor_idx,
            installed,
            search_query,
            editing_query,
            dev_results,
            dev_wizard_step,
            dev_wizard_data,
        );
    } else {
        nav::handle_navigation_or_enter(
            key,
            state,
            context,
            cursor_idx,
            installed,
            search_query,
            editing_query,
            dev_results,
            dev_wizard_step,
            dev_wizard_data,
            left_panel_path,
            right_panel_path,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_editing_query(
    key: KeyEvent,
    state: &mut AppState,
    context: &mut AppContext,
    _cursor_idx: &mut usize,
    installed: &mut Vec<(String, String, bool, bool, Option<String>)>,
    search_query: &mut String,
    editing_query: &mut bool,
    dev_results: &mut String,
    dev_wizard_step: &mut usize,
    dev_wizard_data: &mut Vec<String>,
) {
    match key.code {
        KeyCode::Backspace => {
            search_query.pop();
        }
        KeyCode::Char(c) => {
            search_query.push(c);
        }
        KeyCode::Enter => {
            wizard::handle_wizard_enter(
                state,
                context,
                installed,
                search_query,
                editing_query,
                dev_results,
                dev_wizard_step,
                dev_wizard_data,
            );
        }
        _ => {}
    }
}
