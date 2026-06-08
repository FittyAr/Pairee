use crate::config::settings::Settings;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, bool)>) {
    rows.push((
        format!(
            "[{}] Copy",
            if settings.confirmations.confirm_copy {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Move",
            if settings.confirmations.confirm_move {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Overwrite and delete R/O files",
            if settings.confirmations.confirm_overwrite {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Drag and drop",
            if settings.confirmations.confirm_drag_and_drop {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Delete",
            if settings.confirmations.confirm_delete {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Delete non-empty folders",
            if settings.confirmations.confirm_delete_non_empty_folders {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Interrupt operation",
            if settings.confirmations.confirm_interrupt_operation {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Disconnect network drive",
            if settings.confirmations.confirm_disconnect_network_drive {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Delete SUBST-disk",
            if settings.confirmations.confirm_delete_subst_disk {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Detach virtual disk",
            if settings.confirmations.confirm_detach_virtual_disk {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] HotPlug-device removal",
            if settings.confirmations.confirm_hotplug_removal {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Reload edited file",
            if settings.confirmations.confirm_reload_edited_file {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Clear history list",
            if settings.confirmations.confirm_clear_history_list {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
    rows.push((
        format!(
            "[{}] Exit",
            if settings.confirmations.confirm_quit {
                "x"
            } else {
                " "
            }
        ),
        false,
    ));
}
