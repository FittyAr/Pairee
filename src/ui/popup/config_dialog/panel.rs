use crate::config::settings::Settings;
use crate::config::localization::t;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, bool)>) {
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_hidden { "x" } else { " " },
            t("pan_show_hidden")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.highlight_files { "x" } else { " " },
            t("pan_highlight")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.select_folders { "x" } else { " " },
            t("pan_select_folders")
        ),
        false,
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
        false,
    ));
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
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.sort_reverse { "x" } else { " " },
            t("pan_reverse_sort")
        ),
        false,
    ));
    rows.push((
        format!(
            "{} [ {} ]",
            t("pan_disable_update"),
            settings.disable_panel_update_object_count
        ),
        false,
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
        false,
    ));
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
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_status_line { "x" } else { " " },
            t("pan_status_line")
        ),
        false,
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
        false,
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
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_free_size { "x" } else { " " },
            t("pan_free_size")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.show_scrollbar { "x" } else { " " },
            t("pan_scrollbar")
        ),
        false,
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
        false,
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
        false,
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
        false,
    ));
    rows.push((t("pan_info_settings"), false));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.infopanel_show_power_status {
                "x"
            } else {
                " "
            },
            t("pan_info_power")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.infopanel_show_cd_drive_parameters {
                "x"
            } else {
                " "
            },
            t("pan_info_cd")
        ),
        false,
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("pan_info_computer"),
            settings.infopanel_computer_name_format
        ),
        false,
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("pan_info_user"),
            settings.infopanel_user_name_format
        ),
        false,
    ));
    rows.push((
        t("pan_masks_hint"),
        false,
    ));
    rows.push((
        t("pan_modes_hint"),
        false,
    ));
    rows.push((t("pan_desc_title"), false));
    rows.push((
        format!("  {} [ {} ]", t("pan_desc_names"), settings.file_descriptions_list_names),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.file_descriptions_set_hidden {
                "x"
            } else {
                " "
            },
            t("pan_desc_hidden")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.file_descriptions_update_readonly {
                "x"
            } else {
                " "
            },
            t("pan_desc_readonly")
        ),
        false,
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("pan_desc_pos"),
            settings.file_descriptions_position
        ),
        false,
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("pan_desc_update"),
            settings.file_descriptions_update_mode
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.file_descriptions_use_ansi {
                "x"
            } else {
                " "
            },
            t("pan_desc_ansi")
        ),
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.file_descriptions_save_utf8 {
                "x"
            } else {
                " "
            },
            t("pan_desc_utf8")
        ),
        false,
    ));
    rows.push((
        format!(
            "{} [ {} ]",
            t("pan_folder_desc_names"),
            settings.folder_description_list_names
        ),
        false,
    ));
}
