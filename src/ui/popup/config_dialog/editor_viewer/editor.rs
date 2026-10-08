use super::super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

pub fn populate_editor_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    rows.push(("Editor Settings".to_string(), RowType::Title));
    rows.push((
        format!("  {} [ {} ]", t("ed_tab_size"), settings.editor_tab_size),
        RowType::Setting(10),
    ));
}
