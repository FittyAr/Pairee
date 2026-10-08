use super::super::RowType;
use super::check_row;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_editor_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push((t("ed_internal_title"), RowType::Title));
    rows.push((
        format!("  {} [ {} ]", t("ed_tab_size"), settings.editor_tab_size),
        RowType::Setting(10),
    ));
    rows.push((
        format!(
            "  {} [ {} ]",
            t("ed_expand_tabs"),
            t(settings.editor_expand_tabs.label_key())
        ),
        RowType::Setting(11),
    ));
    rows.push(check_row("ed_auto_indent", settings.editor_auto_indent, 12));
    rows.push(check_row(
        "ed_show_line_numbers",
        settings.editor_show_line_numbers,
        13,
    ));
    rows.push(check_row(
        "ed_cursor_at_end",
        settings.editor_cursor_at_end,
        14,
    ));
    rows.push(check_row(
        "ed_lock_readonly",
        settings.editor_lock_editing_readonly,
        15,
    ));
    rows.push(check_row(
        "ed_warn_readonly",
        settings.editor_warn_opening_readonly,
        16,
    ));
}
