use crate::config::settings::Settings;

pub fn handle_row(
    cursor_idx: usize,
    settings: &mut Settings,
    editing_value: &mut bool,
    edit_buffer: &mut String,
) -> Option<crate::app::state::PopupType> {
    match cursor_idx {
        0 => settings.interface_clock = !settings.interface_clock,
        1 => settings.mouse_support = !settings.mouse_support,
        2 => settings.interface_show_key_bar = !settings.interface_show_key_bar,
        3 => {
            settings.interface_always_show_menu_bar = !settings.interface_always_show_menu_bar;
        }
        4 => {
            settings.interface_screen_saver_minutes = match settings.interface_screen_saver_minutes
            {
                1 => 5,
                5 => 10,
                10 => 15,
                15 => 30,
                30 => 60,
                _ => 1,
            };
        }
        5 => {
            settings.interface_show_total_copy_progress =
                !settings.interface_show_total_copy_progress;
        }
        6 => {
            settings.interface_show_copying_time = !settings.interface_show_copying_time;
        }
        7 => {
            settings.interface_show_total_delete_progress =
                !settings.interface_show_total_delete_progress;
        }
        8 => {
            settings.interface_use_ctrl_pgup_change_drive =
                !settings.interface_use_ctrl_pgup_change_drive;
        }
        9 => {
            settings.interface_use_virtual_terminal = !settings.interface_use_virtual_terminal;
        }
        10 => {
            settings.interface_fullwidth_aware_rendering =
                !settings.interface_fullwidth_aware_rendering;
        }
        11 => {
            settings.interface_cleartype_friendly_redraw =
                !settings.interface_cleartype_friendly_redraw;
        }
        12 => {
            settings.interface_console_icon = match settings.interface_console_icon {
                0 => 1,
                1 => 2,
                _ => 0,
            };
        }
        13 => {
            settings.interface_console_icon_admin_alternate =
                !settings.interface_console_icon_admin_alternate;
        }
        14 => {
            *editing_value = true;
            *edit_buffer = settings.interface_window_title_addons.clone();
        }
        16 => {
            settings.dialog_history_in_edit_controls = !settings.dialog_history_in_edit_controls;
        }
        17 => {
            settings.dialog_persistent_blocks = !settings.dialog_persistent_blocks;
        }
        18 => {
            settings.dialog_del_removes_blocks = !settings.dialog_del_removes_blocks;
        }
        19 => settings.dialog_autocomplete = !settings.dialog_autocomplete,
        20 => {
            settings.dialog_backspace_deletes_unchanged =
                !settings.dialog_backspace_deletes_unchanged;
        }
        21 => {
            settings.dialog_mouse_click_outside_closes =
                !settings.dialog_mouse_click_outside_closes;
        }
        23 => {
            settings.menu_left_click_outside = match settings.menu_left_click_outside.as_str() {
                "Cancel menu" => "Do nothing".to_string(),
                _ => "Cancel menu".to_string(),
            };
        }
        24 => {
            settings.menu_right_click_outside = match settings.menu_right_click_outside.as_str() {
                "Cancel menu" => "Do nothing".to_string(),
                _ => "Cancel menu".to_string(),
            };
        }
        25 => {
            settings.menu_middle_click_outside = match settings.menu_middle_click_outside.as_str() {
                "Execute selected item" => "Cancel menu".to_string(),
                _ => "Execute selected item".to_string(),
            };
        }
        27 => {
            settings.cmdline_persistent_blocks = !settings.cmdline_persistent_blocks;
        }
        28 => {
            settings.cmdline_del_removes_blocks = !settings.cmdline_del_removes_blocks;
        }
        29 => settings.cmdline_autocomplete = !settings.cmdline_autocomplete,
        30 => {
            settings.cmdline_prompt_format = match settings.cmdline_prompt_format.as_str() {
                "$p$g" => "$p".to_string(),
                "$p" => "$g".to_string(),
                _ => "$p$g".to_string(),
            };
        }
        31 => {
            settings.cmdline_use_home_dir = match settings.cmdline_use_home_dir.as_str() {
                "%FARHOME%" => "%USERPROFILE%".to_string(),
                _ => "%FARHOME%".to_string(),
            };
        }
        33 => settings.autocomplete_show_list = !settings.autocomplete_show_list,
        34 => settings.autocomplete_modal_mode = !settings.autocomplete_modal_mode,
        35 => {
            settings.autocomplete_append_first = !settings.autocomplete_append_first;
        }
        36 => {
            settings.keybinding_preset = match settings.keybinding_preset.as_str() {
                "norton" => "neovim".to_string(),
                "neovim" => "vscode".to_string(),
                _ => "norton".to_string(),
            };
        }
        37 => {
            settings.enable_yazi_workflow = !settings.enable_yazi_workflow;
        }
        _ => {}
    }
    None
}
