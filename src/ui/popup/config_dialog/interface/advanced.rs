use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;
use crate::keybindings::loader::load_keybinds;
use std::collections::HashMap;

pub fn populate_advanced_rows(
    settings: &Settings,
    rows: &mut Vec<(String, RowType)>,
    custom_bindings: &HashMap<String, String>,
) {
    // Dialogs
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

    // Menus
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

    // Command line
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

    // Autocomplete
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

    // Keybindings
    rows.push(("Keybindings".to_string(), RowType::Title));
    rows.push((
        format!(
            "{} < {} >",
            t("int_keybindings"),
            settings.keybinding_preset
        ),
        RowType::Setting(36),
    ));
    let (_, keymap_report) = load_keybinds(&settings.keybinding_preset, custom_bindings);
    let status_row = if keymap_report.ok() && keymap_report.warnings.is_empty() {
        RowType::Hint
    } else {
        RowType::Subtitle
    };
    rows.push((keymap_report.summary_line(), status_row));
    rows.push((t("int_keymap_gray"), RowType::Hint));
    rows.push((t("int_keymap_view"), RowType::Setting(38)));
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
