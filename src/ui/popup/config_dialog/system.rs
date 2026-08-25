use super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push(("File Operations".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.delete_to_recycle_bin {
                "x"
            } else {
                " "
            },
            t("sys_delete_recycle")
        ),
        RowType::Setting(0),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.use_system_copy_routine {
                "x"
            } else {
                " "
            },
            t("sys_system_copy")
        ),
        RowType::Setting(1),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.copy_files_opened_for_writing {
                "x"
            } else {
                " "
            },
            t("sys_copy_opened")
        ),
        RowType::Setting(2),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.scan_symbolic_links {
                "x"
            } else {
                " "
            },
            t("sys_scan_symlinks")
        ),
        RowType::Setting(3),
    ));

    rows.push(("History".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.save_commands_history {
                "x"
            } else {
                " "
            },
            t("sys_save_cmd_hist")
        ),
        RowType::Setting(4),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.save_folders_history {
                "x"
            } else {
                " "
            },
            t("sys_save_folder_hist")
        ),
        RowType::Setting(5),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.save_view_and_edit_history {
                "x"
            } else {
                " "
            },
            t("sys_save_view_hist")
        ),
        RowType::Setting(6),
    ));

    rows.push(("Environment".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.use_windows_registered_types {
                "x"
            } else {
                " "
            },
            t("sys_windows_types")
        ),
        RowType::Setting(7),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.automatic_update_env_variables {
                "x"
            } else {
                " "
            },
            t("sys_auto_update_env")
        ),
        RowType::Setting(8),
    ));

    rows.push(("Permissions".to_string(), RowType::Title));
    rows.push((t("sys_req_admin"), RowType::Subtitle));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.req_admin_modification {
                "x"
            } else {
                " "
            },
            t("sys_admin_mod")
        ),
        RowType::Setting(10),
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.req_admin_reading { "x" } else { " " },
            t("sys_admin_read")
        ),
        RowType::Setting(11),
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.req_admin_use_additional_privileges {
                "x"
            } else {
                " "
            },
            t("sys_admin_privs")
        ),
        RowType::Setting(12),
    ));

    rows.push(("Sorting & Saving".to_string(), RowType::Title));
    rows.push((
        format!(
            "{} < {} >",
            t("sys_sort_collation"),
            settings.sorting_collation
        ),
        RowType::Setting(13),
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.treat_digits_as_numbers {
                "x"
            } else {
                " "
            },
            t("sys_digits_numbers")
        ),
        RowType::Setting(14),
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.case_sensitive_sort {
                "x"
            } else {
                " "
            },
            t("sys_case_sensitive")
        ),
        RowType::Setting(15),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.auto_save_setup { "x" } else { " " },
            t("sys_auto_save")
        ),
        RowType::Setting(16),
    ));

    rows.push((t("feature_section"), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.ssh_enabled { "x" } else { " " },
            t("feature_ssh")
        ),
        RowType::Setting(17),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.plugins_enabled { "x" } else { " " },
            t("feature_plugins")
        ),
        RowType::Setting(18),
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.image_preview_enabled {
                "x"
            } else {
                " "
            },
            t("feature_image_preview")
        ),
        RowType::Setting(19),
    ));
}
