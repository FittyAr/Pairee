use super::RowType;
use crate::config::settings::Settings;

mod editor;
mod viewer;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    editor::populate_editor_rows(settings, rows);
    viewer::populate_viewer_rows(settings, rows);
}
