use super::RowType;
use crate::config::settings::Settings;

mod editor;
mod viewer;

pub fn populate_rows(
    settings: &Settings,
    editing_value: bool,
    cursor_idx: usize,
    edit_buffer: &str,
    rows: &mut Vec<(String, RowType)>,
) {
    editor::populate_editor_rows(settings, editing_value, cursor_idx, edit_buffer, rows);
    viewer::populate_viewer_rows(settings, editing_value, cursor_idx, edit_buffer, rows);
}
