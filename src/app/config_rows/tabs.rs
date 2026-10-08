//! System, Panel and Confirmations tabs.

use super::{CycleFormat, Label, Row, cycle, title, toggle};

pub fn system() -> Vec<Row> {
    vec![
        title("File Operations"),
        toggle!("sys_delete_recycle", delete_to_recycle_bin),
        title("History"),
        toggle!("sys_save_cmd_hist", save_commands_history),
        toggle!("sys_save_folder_hist", save_folders_history),
        toggle!("sys_save_view_hist", save_view_and_edit_history),
        title("Environment"),
        toggle!("sys_windows_types", use_windows_registered_types),
        toggle!("sys_auto_update_env", automatic_update_env_variables),
        title("Permissions"),
        Row::Subtitle(Label::Key("sys_req_admin")),
        toggle!(1, "sys_admin_mod", req_admin_modification),
        toggle!(1, "sys_admin_read", req_admin_reading),
        title("Sorting & Saving"),
        cycle(
            "sys_sort_collation",
            0,
            CycleFormat::Angle,
            |s| s.sorting_collation.clone(),
            |s| {
                s.sorting_collation = match s.sorting_collation.as_str() {
                    "linguistic" => "natural",
                    _ => "linguistic",
                }
                .to_string();
            },
        ),
        toggle!(1, "sys_digits_numbers", treat_digits_as_numbers),
        toggle!(1, "sys_case_sensitive", case_sensitive_sort),
        toggle!("sys_auto_save", auto_save_setup),
        super::title_key("feature_section"),
        toggle!("feature_ssh", ssh_enabled),
        toggle!("feature_plugins", plugins_enabled),
        toggle!("feature_image_preview", image_preview_enabled),
    ]
}

pub fn panel() -> Vec<Row> {
    vec![
        toggle!("pan_show_hidden", show_hidden),
        toggle!("pan_highlight", highlight_files),
        toggle!("pan_select_folders", select_folders),
        title("Sorting"),
        toggle!("pan_sort_folders_ext", sort_folder_names_by_extension),
        toggle!("pan_reverse_sort", sort_reverse),
        toggle!("pan_sort_letter", show_sort_mode_letter),
        title("Updates & Information"),
        toggle!("pan_auto_refresh", auto_refresh),
        cycle(
            "pan_auto_refresh_poll",
            1,
            CycleFormat::Bracket,
            |s| s.auto_refresh_poll_secs.to_string(),
            |s| {
                s.auto_refresh_poll_secs = match s.auto_refresh_poll_secs {
                    0..=2 => 3,
                    3..=4 => 5,
                    5..=9 => 10,
                    10..=29 => 30,
                    _ => 2,
                };
            },
        ),
        cycle(
            "pan_disable_update",
            0,
            CycleFormat::Bracket,
            |s| s.disable_panel_update_object_count.to_string(),
            |s| {
                s.disable_panel_update_object_count = match s.disable_panel_update_object_count {
                    0 => 100,
                    100 => 1000,
                    1000 => 10000,
                    _ => 0,
                };
            },
        ),
        toggle!("pan_total_info", show_files_total_information),
        toggle!("pan_free_size", show_free_size),
        title("Appearance"),
        toggle!("pan_col_titles", show_column_titles),
        toggle!("pan_status_line", show_status_line),
        toggle!("pan_scrollbar", show_scrollbar),
        toggle!("pan_dotdot_root", show_dotdot_in_root_folders),
        toggle!("pan_always_tab_bar", always_show_tab_bar),
    ]
}

pub fn confirmations() -> Vec<Row> {
    vec![
        title("File Operations"),
        toggle!("conf_copy", confirmations.confirm_copy),
        toggle!("conf_move", confirmations.confirm_move),
        toggle!("conf_delete", confirmations.confirm_delete),
        toggle!(
            "conf_delete_non_empty",
            confirmations.confirm_delete_non_empty_folders
        ),
        title("Drives & System"),
        toggle!("conf_interrupt", confirmations.confirm_interrupt_operation),
        title("General"),
        toggle!("conf_reload", confirmations.confirm_reload_edited_file),
        toggle!(
            "conf_clear_history",
            confirmations.confirm_clear_history_list
        ),
        toggle!("conf_exit", confirmations.confirm_quit),
    ]
}
