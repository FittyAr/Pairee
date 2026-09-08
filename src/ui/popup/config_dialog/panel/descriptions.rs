use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_descriptions(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push(("Info Panel Settings".to_string(), RowType::Title));
    // t("pan_info_settings") was index 17
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
        RowType::Setting(18),
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
        RowType::Setting(19),
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("pan_info_computer"),
            settings.infopanel_computer_name_format
        ),
        RowType::Setting(20),
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("pan_info_user"),
            settings.infopanel_user_name_format
        ),
        RowType::Setting(21),
    ));

    rows.push(("File Descriptions".to_string(), RowType::Title));
    rows.push((t("pan_masks_hint"), RowType::Hint)); // 22
    rows.push((t("pan_modes_hint"), RowType::Hint)); // 23
    // t("pan_desc_title") was index 24
    rows.push((
        format!(
            "  {} [ {} ]",
            t("pan_desc_names"),
            settings.file_descriptions_list_names
        ),
        RowType::Setting(25),
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
        RowType::Setting(26),
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
        RowType::Setting(27),
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("pan_desc_pos"),
            settings.file_descriptions_position
        ),
        RowType::Setting(28),
    ));
    rows.push((
        format!(
            "  {} < {} >",
            t("pan_desc_update"),
            settings.file_descriptions_update_mode
        ),
        RowType::Setting(29),
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
        RowType::Setting(30),
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
        RowType::Setting(31),
    ));
    rows.push((
        format!(
            "{} [ {} ]",
            t("pan_folder_desc_names"),
            settings.folder_description_list_names
        ),
        RowType::Setting(32),
    ));
}
