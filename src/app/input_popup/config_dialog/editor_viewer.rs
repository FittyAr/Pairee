use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    editing_value: &mut bool,
    edit_buffer: &mut String,
) -> Option<crate::app::state::PopupType> {
    match cursor_idx {
        0 => settings.editor_use_external = !settings.editor_use_external,
        1 => {
            *editing_value = true;
            *edit_buffer = settings.default_editor.clone();
        }
        3 => {
            settings.editor_expand_tabs = match settings.editor_expand_tabs.as_str() {
                "Do not expand tabs" => "Expand tabs".to_string(),
                _ => "Do not expand tabs".to_string(),
            };
        }
        4 => settings.editor_persistent_blocks = !settings.editor_persistent_blocks,
        5 => settings.editor_cursor_beyond_eol = !settings.editor_cursor_beyond_eol,
        6 => {
            settings.editor_del_removes_blocks = !settings.editor_del_removes_blocks;
        }
        7 => settings.editor_select_found = !settings.editor_select_found,
        8 => settings.editor_auto_indent = !settings.editor_auto_indent,
        9 => settings.editor_cursor_at_end = !settings.editor_cursor_at_end,
        10 => {
            settings.editor_tab_size = match settings.editor_tab_size {
                2 => 4,
                4 => 8,
                _ => 2,
            };
        }
        11 => settings.editor_show_scrollbar = !settings.editor_show_scrollbar,
        12 => settings.editor_show_white_space = !settings.editor_show_white_space,
        13 => {
            settings.editor_show_line_numbers = !settings.editor_show_line_numbers;
        }
        14 => {
            settings.editor_save_file_position = !settings.editor_save_file_position;
        }
        15 => settings.editor_save_bookmarks = !settings.editor_save_bookmarks,
        16 => {
            settings.editor_allow_editing_opened_writing =
                !settings.editor_allow_editing_opened_writing;
        }
        17 => {
            settings.editor_lock_editing_readonly = !settings.editor_lock_editing_readonly;
        }
        18 => {
            settings.editor_warn_opening_readonly = !settings.editor_warn_opening_readonly;
        }
        19 => {
            settings.editor_autodetect_codepage = !settings.editor_autodetect_codepage;
        }
        20 => {
            settings.editor_default_codepage = match settings.editor_default_codepage.as_str() {
                "1252" => "65001".to_string(),
                "65001" => "1200".to_string(),
                _ => "1252".to_string(),
            };
        }
        21 => settings.viewer_use_external = !settings.viewer_use_external,
        22 => {
            *editing_value = true;
            *edit_buffer = settings.viewer_command.clone();
        }
        24 => {
            settings.viewer_persistent_selection = !settings.viewer_persistent_selection;
        }
        25 => {
            settings.viewer_show_scrolling_arrows = !settings.viewer_show_scrolling_arrows;
        }
        26 => {
            settings.viewer_tab_size = match settings.viewer_tab_size {
                2 => 4,
                4 => 8,
                _ => 2,
            };
        }
        27 => settings.viewer_visible_zero = !settings.viewer_visible_zero,
        28 => settings.viewer_show_scrollbar = !settings.viewer_show_scrollbar,
        29 => {
            settings.viewer_save_file_position = !settings.viewer_save_file_position;
        }
        30 => settings.viewer_save_view_mode = !settings.viewer_save_view_mode,
        31 => {
            settings.viewer_save_file_codepage = !settings.viewer_save_file_codepage;
        }
        32 => settings.viewer_save_wrap_mode = !settings.viewer_save_wrap_mode,
        33 => settings.viewer_save_bookmarks = !settings.viewer_save_bookmarks,
        34 => {
            settings.viewer_detect_dump_view_mode = !settings.viewer_detect_dump_view_mode;
        }
        35 => {
            settings.viewer_max_line_width = match settings.viewer_max_line_width {
                1000 => 10000,
                10000 => 50000,
                _ => 1000,
            };
        }
        36 => {
            settings.viewer_autodetect_codepage = !settings.viewer_autodetect_codepage;
        }
        37 => {
            settings.viewer_default_codepage = match settings.viewer_default_codepage.as_str() {
                "1252" => "65001".to_string(),
                "65001" => "1200".to_string(),
                _ => "1252".to_string(),
            };
        }
        38 => settings.enter_use_external = !settings.enter_use_external,
        _ => {}
    }
    None
}
