use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    editing_value: &mut bool,
    edit_buffer: &mut String,
) -> Option<crate::app::state::PopupType> {
    match cursor_idx {
        0 => {
            let discovered = crate::config::localization::discover_languages();
            if !discovered.is_empty() {
                let current_idx = discovered
                    .iter()
                    .position(|(name, _)| name == &settings.language);
                let next_idx = match current_idx {
                    Some(idx) => (idx + 1) % discovered.len(),
                    None => 0,
                };
                settings.language = discovered[next_idx].0.clone();
            }
        }
        11 => {
            settings.plugins_developer_mode = !settings.plugins_developer_mode;
        }
        12 => {
            *editing_value = true;
            *edit_buffer = settings.plugins_dev_dir.clone();
        }
        _ => {}
    }
    None
}
