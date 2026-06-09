use crate::config::settings::Settings;
use crate::config::localization::t;

pub fn populate_rows(
    settings: &Settings,
    editing_value: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, bool)>,
) {
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_clock { "x" } else { " " },
            t("int_clock")
        ),
        false,
    ));
    rows.push((
        format!("[{}] {}", if settings.mouse_support { "x" } else { " " }, t("int_mouse")),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_show_key_bar {
                "x"
            } else {
                " "
            },
            t("int_key_bar")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_always_show_menu_bar {
                "x"
            } else {
                " "
            },
            t("int_menu_bar")
        ),
        false,
    ));
    rows.push((
        format!(
            "{} [ {} ] minutes",
            t("int_screensaver"),
            settings.interface_screen_saver_minutes
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_show_total_copy_progress {
                "x"
            } else {
                " "
            },
            t("int_copy_progress")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_show_copying_time {
                "x"
            } else {
                " "
            },
            t("int_copy_time")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_show_total_delete_progress {
                "x"
            } else {
                " "
            },
            t("int_delete_progress")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_use_ctrl_pgup_change_drive {
                "x"
            } else {
                " "
            },
            t("int_ctrl_pgup")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_use_virtual_terminal {
                "x"
            } else {
                " "
            },
            t("int_vt")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_fullwidth_aware_rendering {
                "x"
            } else {
                " "
            },
            t("int_fullwidth")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_cleartype_friendly_redraw {
                "x"
            } else {
                " "
            },
            t("int_cleartype")
        ),
        false,
    ));
    rows.push((
        format!("{} [ {} ]", t("int_icon"), settings.interface_console_icon),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.interface_console_icon_admin_alternate {
                "x"
            } else {
                " "
            },
            t("int_icon_admin")
        ),
        false,
    ));
    if editing_value && cursor_idx == 14 {
        rows.push((
            format!("{} [ {}◄ ]", t("int_title_addons"), edit_buffer),
            false,
        ));
    } else {
        rows.push((
            format!(
                "{} [ {} ]",
                t("int_title_addons"),
                settings.interface_window_title_addons
            ),
            false,
        ));
    }
    rows.push((t("int_diag_title"), false));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.dialog_history_in_edit_controls {
                "x"
            } else {
                " "
            },
            t("int_diag_history")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.dialog_persistent_blocks {
                "x"
            } else {
                " "
            },
            t("int_diag_blocks")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.dialog_del_removes_blocks {
                "x"
            } else {
                " "
            },
            t("int_diag_del")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.dialog_autocomplete {
                "x"
            } else {
                " "
            },
            t("int_diag_auto")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.dialog_backspace_deletes_unchanged {
                "x"
            } else {
                " "
            },
            t("int_diag_backspace")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.dialog_mouse_click_outside_closes {
                "x"
            } else {
                " "
            },
            t("int_diag_mouse")
        ),
        false,
    ));
    rows.push((t("int_menu_title"), false));
    rows.push((
        format!(
            "  {} < {} >",
            t("int_menu_left"),
            settings.menu_left_click_outside
        ),
        false,
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("int_menu_right"),
            settings.menu_right_click_outside
        ),
        false,
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("int_menu_middle"),
            settings.menu_middle_click_outside
        ),
        false,
    ));
    rows.push((t("int_cmd_title"), false));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.cmdline_persistent_blocks {
                "x"
            } else {
                " "
            },
            t("int_cmd_blocks")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.cmdline_del_removes_blocks {
                "x"
            } else {
                " "
            },
            t("int_cmd_del")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.cmdline_autocomplete {
                "x"
            } else {
                " "
            },
            t("int_cmd_auto")
        ),
        false,
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("int_cmd_prompt"),
            settings.cmdline_prompt_format
        ),
        false,
    ));
    rows.push((
        format!("  {} [ {} ]", t("int_cmd_home"), settings.cmdline_use_home_dir),
        false,
    ));
    rows.push((t("int_auto_title"), false));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.autocomplete_show_list {
                "x"
            } else {
                " "
            },
            t("int_auto_list")
        ),
        false,
    ));
    rows.push((
        format!(
            "    [{}] {}",
            if settings.autocomplete_modal_mode {
                "x"
            } else {
                " "
            },
            t("int_auto_modal")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.autocomplete_append_first {
                "x"
            } else {
                " "
            },
            t("int_auto_append")
        ),
        false,
    ));
    rows.push((
        format!("{} < {} >", t("int_keybindings"), settings.keybinding_preset),
        false,
    ));
}
