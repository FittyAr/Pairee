use super::RowType;
use crate::config::localization::t;
use crate::config::settings::Settings;

mod editor;
mod viewer;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    editor::populate_editor_rows(settings, rows);
    viewer::populate_viewer_rows(settings, rows);
}

/// A `[x] label` checkbox row bound to setting `id`.
fn check_row(label_key: &str, checked: bool, id: usize) -> (String, RowType) {
    (
        format!("  [{}] {}", if checked { "x" } else { " " }, t(label_key)),
        RowType::Setting(id),
    )
}
