use super::{KeyGrid, key_rows};
use ratatui::widgets::Row;

/// Sort keys of the Yazi-style popup, three rows of four.
const SORT_KEYS: KeyGrid = [
    [
        Some((" n ", " Name")),
        Some((" e ", " Extension")),
        Some((" s ", " Size")),
        Some((" w ", " Write time")),
    ],
    [
        Some((" c ", " Creation time")),
        Some((" a ", " Access time")),
        Some((" d ", " Description")),
        Some((" o ", " Owner")),
    ],
    [
        Some((" u ", " Unsorted")),
        Some((" r ", " Reverse order")),
        None,
        None,
    ],
];

pub fn build_sort_rows(theme: &crate::config::theme::Theme) -> Vec<Row<'static>> {
    key_rows(theme, &SORT_KEYS)
}
