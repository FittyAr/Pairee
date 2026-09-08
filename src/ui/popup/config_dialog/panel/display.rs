use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_display(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_hidden { "x" } else { " " },
            t("pan_show_hidden")
        ),
        RowType::Setting(0),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.highlight_files { "x" } else { " " },
            t("pan_highlight")
        ),
        RowType::Setting(1),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.select_folders { "x" } else { " " },
            t("pan_select_folders")
        ),
        RowType::Setting(2),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.right_click_selects_files {
                "x"
            } else {
                " "
            },
            t("pan_right_click")
        ),
        RowType::Setting(3),
    ));

    rows.push(("Sorting".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.sort_folder_names_by_extension {
                "x"
            } else {
                " "
            },
            t("pan_sort_folders_ext")
        ),
        RowType::Setting(4),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.sort_reverse { "x" } else { " " },
            t("pan_reverse_sort")
        ),
        RowType::Setting(5),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_sort_mode_letter {
                "x"
            } else {
                " "
            },
            t("pan_sort_letter")
        ),
        RowType::Setting(15),
    ));

    rows.push(("Updates & Information".to_string(), RowType::Title));
    rows.push((
        format!(
            "{} [ {} ]",
            t("pan_disable_update"),
            settings.disable_panel_update_object_count
        ),
        RowType::Setting(6),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.network_drives_autorefresh {
                "x"
            } else {
                " "
            },
            t("pan_net_refresh")
        ),
        RowType::Setting(7),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.detect_volume_mount_points {
                "x"
            } else {
                " "
            },
            t("pan_volume_points")
        ),
        RowType::Setting(10),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_files_total_information {
                "x"
            } else {
                " "
            },
            t("pan_total_info")
        ),
        RowType::Setting(11),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_free_size { "x" } else { " " },
            t("pan_free_size")
        ),
        RowType::Setting(12),
    ));

    rows.push(("Appearance".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_column_titles {
                "x"
            } else {
                " "
            },
            t("pan_col_titles")
        ),
        RowType::Setting(8),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_status_line { "x" } else { " " },
            t("pan_status_line")
        ),
        RowType::Setting(9),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_scrollbar { "x" } else { " " },
            t("pan_scrollbar")
        ),
        RowType::Setting(13),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_background_screens_number {
                "x"
            } else {
                " "
            },
            t("pan_bg_screens")
        ),
        RowType::Setting(14),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_dotdot_in_root_folders {
                "x"
            } else {
                " "
            },
            t("pan_dotdot_root")
        ),
        RowType::Setting(16),
    ));
}
