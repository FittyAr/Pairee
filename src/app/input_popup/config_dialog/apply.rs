use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::app::sys_helpers::{change_preset, change_theme};
use crate::config::settings::Settings;

pub fn apply_settings(state: &mut AppState, context: &mut AppContext, settings: Settings) {
    if settings.theme != context.config.settings.theme {
        change_theme(context, state, &settings.theme);
    }
    if settings.keybinding_preset != context.config.settings.keybinding_preset {
        change_preset(context, &settings.keybinding_preset);
    }
    state.case_sensitive_sort = settings.case_sensitive_sort;
    state.treat_digits_as_numbers = settings.treat_digits_as_numbers;
    state.sorting_collation = settings.sorting_collation.clone();
    state.req_admin_reading = settings.req_admin_reading;
    // Panel settings
    state.select_folders = settings.select_folders;
    state.sort_folder_names_by_extension = settings.sort_folder_names_by_extension;
    state.show_dotdot_in_root_folders = settings.show_dotdot_in_root_folders;
    state.disable_panel_update_object_count = settings.disable_panel_update_object_count;
    let lang_to_load = settings.language.clone();
    context.config.settings = settings;
    let _ = context.config.save();
    crate::config::localization::load_language(&lang_to_load);
    state.refresh_both_panels(context.config.settings.show_hidden);
    state.active_popup = None;
}
