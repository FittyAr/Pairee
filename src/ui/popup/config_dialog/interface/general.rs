use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_general_rows(
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
}
