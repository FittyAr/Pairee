use crate::config::settings::Settings;
use crate::ui::popup::config_dialog::RowType;

pub fn get_rows_for_tab(
    tab: usize,
    settings: &Settings,
    editing: bool,
    c_idx: usize,
    buf: &str,
) -> Vec<(String, RowType)> {
    let mut rows = Vec::new();
    match tab {
        0 => crate::ui::popup::config_dialog::system::populate_rows(settings, &mut rows),
        1 => crate::ui::popup::config_dialog::panel::populate_rows(settings, &mut rows),
        2 => crate::ui::popup::config_dialog::interface::populate_rows(
            settings, editing, c_idx, buf, &mut rows,
        ),
        3 => crate::ui::popup::config_dialog::confirmations::populate_rows(settings, &mut rows),
        4 => crate::ui::popup::config_dialog::plugins::populate_rows(settings, &mut rows),
        5 => crate::ui::popup::config_dialog::editor_viewer::populate_rows(
            settings, editing, c_idx, buf, &mut rows,
        ),
        6 => crate::ui::popup::config_dialog::colors::populate_rows(settings, &mut rows),
        7 => crate::ui::popup::config_dialog::git::populate_rows(
            settings, editing, c_idx, buf, &mut rows,
        ),
        _ => {}
    }
    rows
}

pub fn is_selectable(idx: usize, rows: &[(String, RowType)]) -> bool {
    if idx >= rows.len() {
        true // OK and Cancel buttons
    } else {
        matches!(rows[idx].1, RowType::Setting(_))
    }
}
