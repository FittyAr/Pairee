use crate::config::settings::Settings;
use crate::config::localization::t;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, bool)>) {
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
        false,
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
        false,
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
        false,
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
        false,
    ));
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
        false,
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
        false,
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
        false,
    ));
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
        false,
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
        false,
    ));
    rows.push((t("sys_req_admin"), false));
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
        false,
    ));
    rows.push((
        format!(
            "  [{}] {}",
            if settings.req_admin_reading { "x" } else { " " },
            t("sys_admin_read")
        ),
        false,
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
        false,
    ));
    rows.push((
        format!("{} < {} >", t("sys_sort_collation"), settings.sorting_collation),
        false,
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
        false,
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
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.auto_save_setup { "x" } else { " " },
            t("sys_auto_save")
        ),
        false,
    ));
}
