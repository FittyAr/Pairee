mod descriptions;
mod display;

use super::RowType;
use crate::config::settings::Settings;

pub fn populate_rows(settings: &Settings, rows: &mut Vec<(String, RowType)>) {
    display::populate_display(settings, rows);
    descriptions::populate_descriptions(settings, rows);
}
