use crate::config::settings::Settings;
use crate::config::localization::t;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, bool)>) {
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_copy {
                "x"
            } else {
                " "
            },
            t("conf_copy")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_move {
                "x"
            } else {
                " "
            },
            t("conf_move")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_overwrite {
                "x"
            } else {
                " "
            },
            t("conf_overwrite")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_drag_and_drop {
                "x"
            } else {
                " "
            },
            t("conf_drag_drop")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_delete {
                "x"
            } else {
                " "
            },
            t("conf_delete")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_delete_non_empty_folders {
                "x"
            } else {
                " "
            },
            t("conf_delete_non_empty")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_interrupt_operation {
                "x"
            } else {
                " "
            },
            t("conf_interrupt")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_disconnect_network_drive {
                "x"
            } else {
                " "
            },
            t("conf_disconnect")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_delete_subst_disk {
                "x"
            } else {
                " "
            },
            t("conf_delete_subst")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_detach_virtual_disk {
                "x"
            } else {
                " "
            },
            t("conf_detach_vdisk")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_hotplug_removal {
                "x"
            } else {
                " "
            },
            t("conf_hotplug")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_reload_edited_file {
                "x"
            } else {
                " "
            },
            t("conf_reload")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_clear_history_list {
                "x"
            } else {
                " "
            },
            t("conf_clear_history")
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] {}",
            if settings.confirmations.confirm_quit {
                "x"
            } else {
                " "
            },
            t("conf_exit")
        ),
        false,
    ));
}
