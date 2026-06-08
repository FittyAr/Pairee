use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    _editing_value: &mut bool,
    _edit_buffer: &mut String,
) {
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
        3 => {
            settings.plugins_manager_oem_support = !settings.plugins_manager_oem_support;
        }
        4 => {
            settings.plugins_manager_scan_symlinks = !settings.plugins_manager_scan_symlinks;
        }
        6 => {
            settings.plugins_manager_file_processing = !settings.plugins_manager_file_processing;
        }
        7 => {
            settings.plugins_manager_show_standard_association =
                !settings.plugins_manager_show_standard_association;
        }
        8 => {
            settings.plugins_manager_even_if_one_found =
                !settings.plugins_manager_even_if_one_found;
        }
        9 => {
            settings.plugins_manager_search_results = !settings.plugins_manager_search_results;
        }
        10 => {
            settings.plugins_manager_prefix_processing =
                !settings.plugins_manager_prefix_processing;
        }
        _ => {}
    }
}
