use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    _editing_value: &mut bool,
    _edit_buffer: &mut String,
) -> Option<crate::app::state::PopupType> {
    match cursor_idx {
        0 => settings.show_hidden = !settings.show_hidden,
        1 => settings.highlight_files = !settings.highlight_files,
        2 => settings.select_folders = !settings.select_folders,
        4 => {
            settings.sort_folder_names_by_extension = !settings.sort_folder_names_by_extension;
        }
        5 => settings.sort_reverse = !settings.sort_reverse,
        6 => {
            settings.disable_panel_update_object_count =
                match settings.disable_panel_update_object_count {
                    0 => 100,
                    100 => 1000,
                    1000 => 10000,
                    _ => 0,
                };
        }
        8 => settings.show_column_titles = !settings.show_column_titles,
        9 => settings.show_status_line = !settings.show_status_line,
        11 => {
            settings.show_files_total_information = !settings.show_files_total_information;
        }
        12 => settings.show_free_size = !settings.show_free_size,
        13 => settings.show_scrollbar = !settings.show_scrollbar,
        15 => settings.show_sort_mode_letter = !settings.show_sort_mode_letter,
        16 => {
            settings.show_dotdot_in_root_folders = !settings.show_dotdot_in_root_folders;
        }
        _ => {}
    }
    None
}
