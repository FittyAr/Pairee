use crate::app::state::PopupType;
use crate::config::settings::Settings;

/// Handles row selection in the Git configuration tab.
/// `setting_id` is the ID field from `RowType::Setting(id)`.
pub fn handle_row(
    setting_id: usize,
    settings: &mut Settings,
    editing_value: &mut bool,
    edit_buffer: &mut String,
) -> Option<PopupType> {
    match setting_id {
        // 800: Toggle git_enabled
        800 => {
            settings.git_enabled = !settings.git_enabled;
            None
        }
        // 801: Toggle git_auto_detect
        801 => {
            settings.git_auto_detect = !settings.git_auto_detect;
            None
        }
        // 802: Edit git_author_name
        802 => {
            *editing_value = true;
            *edit_buffer = settings.git_author_name.clone();
            None
        }
        // 803: Edit git_author_email
        803 => {
            *editing_value = true;
            *edit_buffer = settings.git_author_email.clone();
            None
        }
        // 804: Edit git_log_limit
        804 => {
            *editing_value = true;
            *edit_buffer = settings.git_log_limit.to_string();
            None
        }
        _ => None,
    }
}

/// Called when confirming a text field edit (Enter key) for the Git tab.
pub fn apply_edit(setting_id: usize, settings: &mut Settings, edit_buffer: &str) {
    match setting_id {
        802 => settings.git_author_name = edit_buffer.to_string(),
        803 => settings.git_author_email = edit_buffer.to_string(),
        804 => {
            if let Ok(n) = edit_buffer.parse::<u32>() {
                settings.git_log_limit = n.max(1).min(10_000);
            }
        }
        _ => {}
    }
}
