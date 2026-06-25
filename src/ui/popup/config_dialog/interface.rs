use super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_rows(
    settings: &Settings,
    editing_value: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, RowType)>,
) {
    rows.push(("General".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.interface_clock { "x" } else { " " },
            t("int_clock")
        ),
        RowType::Setting(0),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.mouse_support { "x" } else { " " },
            t("int_mouse")
        ),
        RowType::Setting(1),
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
        RowType::Setting(2),
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
        RowType::Setting(3),
    ));
    rows.push((
        format!(
            "{} [ {} ] minutes",
            t("int_screensaver"),
            settings.interface_screen_saver_minutes
        ),
        RowType::Setting(4),
    ));

    rows.push(("Progress Indicators".to_string(), RowType::Title));
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
        RowType::Setting(5),
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
        RowType::Setting(6),
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
        RowType::Setting(7),
    ));

    rows.push(("Terminal & Rendering".to_string(), RowType::Title));
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
        RowType::Setting(8),
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
        RowType::Setting(9),
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
        RowType::Setting(10),
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
        RowType::Setting(11),
    ));

    rows.push(("Window".to_string(), RowType::Title));
    rows.push((
        format!("{} [ {} ]", t("int_icon"), settings.interface_console_icon),
        RowType::Setting(12),
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
        RowType::Setting(13),
    ));

    let is_editing_title = editing_value && cursor_idx == rows.len();
    if is_editing_title {
        rows.push((
            format!("{} [ {}◄ ]", t("int_title_addons"), edit_buffer),
            RowType::Setting(14),
        ));
    } else {
        rows.push((
            format!(
                "{} [ {} ]",
                t("int_title_addons"),
                settings.interface_window_title_addons
            ),
            RowType::Setting(14),
        ));
    }

    // t("int_diag_title") was index 15
    rows.push((t("int_diag_title"), RowType::Title));
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
        RowType::Setting(16),
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
        RowType::Setting(17),
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
        RowType::Setting(18),
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
        RowType::Setting(19),
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
        RowType::Setting(20),
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
        RowType::Setting(21),
    ));

    // t("int_menu_title") was index 22
    rows.push((t("int_menu_title"), RowType::Title));
    rows.push((
        format!(
            "  {} < {} >",
            t("int_menu_left"),
            settings.menu_left_click_outside
        ),
        RowType::Setting(23),
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("int_menu_right"),
            settings.menu_right_click_outside
        ),
        RowType::Setting(24),
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("int_menu_middle"),
            settings.menu_middle_click_outside
        ),
        RowType::Setting(25),
    ));

    // t("int_cmd_title") was index 26
    rows.push((t("int_cmd_title"), RowType::Title));
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
        RowType::Setting(27),
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
        RowType::Setting(28),
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
        RowType::Setting(29),
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("int_cmd_prompt"),
            settings.cmdline_prompt_format
        ),
        RowType::Setting(30),
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("int_cmd_home"),
            settings.cmdline_use_home_dir
        ),
        RowType::Setting(31),
    ));

    // t("int_auto_title") was index 32
    rows.push((t("int_auto_title"), RowType::Title));
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
        RowType::Setting(33),
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
        RowType::Setting(34),
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
        RowType::Setting(35),
    ));

    rows.push(("Keybindings".to_string(), RowType::Title));
    rows.push((
        format!(
            "{} < {} >",
            t("int_keybindings"),
            settings.keybinding_preset
        ),
        RowType::Setting(36),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.enable_yazi_workflow {
                "x"
            } else {
                " "
            },
            t("int_yazi_workflow")
        ),
        RowType::Setting(37),
    ));
}
