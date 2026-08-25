use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    _editing_value: &mut bool,
    _edit_buffer: &mut String,
) -> Option<crate::app::state::PopupType> {
    match cursor_idx {
        0 => settings.delete_to_recycle_bin = !settings.delete_to_recycle_bin,
        1 => settings.use_system_copy_routine = !settings.use_system_copy_routine,
        2 => {
            settings.copy_files_opened_for_writing = !settings.copy_files_opened_for_writing;
        }
        3 => settings.scan_symbolic_links = !settings.scan_symbolic_links,
        4 => settings.save_commands_history = !settings.save_commands_history,
        5 => settings.save_folders_history = !settings.save_folders_history,
        6 => {
            settings.save_view_and_edit_history = !settings.save_view_and_edit_history;
        }
        7 => {
            settings.use_windows_registered_types = !settings.use_windows_registered_types;
        }
        8 => {
            settings.automatic_update_env_variables = !settings.automatic_update_env_variables;
        }
        10 => settings.req_admin_modification = !settings.req_admin_modification,
        11 => settings.req_admin_reading = !settings.req_admin_reading,
        12 => {
            settings.req_admin_use_additional_privileges =
                !settings.req_admin_use_additional_privileges;
        }
        13 => {
            settings.sorting_collation = match settings.sorting_collation.as_str() {
                "linguistic" => "natural".to_string(),
                _ => "linguistic".to_string(),
            };
        }
        14 => settings.treat_digits_as_numbers = !settings.treat_digits_as_numbers,
        15 => settings.case_sensitive_sort = !settings.case_sensitive_sort,
        16 => settings.auto_save_setup = !settings.auto_save_setup,
        17 => settings.ssh_enabled = !settings.ssh_enabled,
        18 => settings.plugins_enabled = !settings.plugins_enabled,
        19 => settings.image_preview_enabled = !settings.image_preview_enabled,
        _ => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_feature_ids_toggle_flags() {
        let mut settings = Settings::default();
        assert!(settings.ssh_enabled);
        handle_row(17, &mut settings, &mut false, &mut String::new());
        assert!(!settings.ssh_enabled);
        handle_row(18, &mut settings, &mut false, &mut String::new());
        assert!(!settings.plugins_enabled);
        handle_row(19, &mut settings, &mut false, &mut String::new());
        assert!(!settings.image_preview_enabled);
    }
}
