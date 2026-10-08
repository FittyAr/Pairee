use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_viewer_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push(("Viewer Settings".to_string(), RowType::Title));
    rows.push((
        format!(
            "[{}] {}",
            if settings.viewer_use_external {
                "x"
            } else {
                " "
            },
            t("vi_external")
        ),
        RowType::Setting(21),
    ));

    rows.push((
        format!(
            "  [{}] {}",
            if settings.enter_use_external {
                "x"
            } else {
                " "
            },
            t("vi_enter_external")
        ),
        RowType::Setting(38),
    ));

    rows.push((t("vi_internal_title"), RowType::Subtitle));
    rows.push((
        format!("  {} [ {} ]", t("vi_tab_size"), settings.viewer_tab_size),
        RowType::Setting(26),
    ));

    rows.push((
        format!(
            "  [{}] {}",
            if settings.viewer_show_scrollbar {
                "x"
            } else {
                " "
            },
            t("vi_show_scrollbar")
        ),
        RowType::Setting(28),
    ));
}
