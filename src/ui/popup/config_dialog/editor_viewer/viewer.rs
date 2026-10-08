use super::super::RowType;
use super::check_row;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_viewer_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push((t("vi_settings_title"), RowType::Title));
    rows.push(check_row("vi_external", settings.viewer_use_external, 21));
    rows.push(check_row(
        "vi_enter_external",
        settings.enter_use_external,
        38,
    ));

    rows.push((t("vi_internal_title"), RowType::Subtitle));
    rows.push((
        format!("  {} [ {} ]", t("vi_tab_size"), settings.viewer_tab_size),
        RowType::Setting(26),
    ));
    rows.push(check_row(
        "vi_show_scrollbar",
        settings.viewer_show_scrollbar,
        28,
    ));
}
