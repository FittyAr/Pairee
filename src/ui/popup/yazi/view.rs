use super::KeyGrid;

/// View-mode keys of the Yazi-style popup, three rows of four.
pub const VIEW_KEYS: KeyGrid = [
    [
        Some((" 1/b ", "yazi_view_brief")),
        Some((" 2/m ", "yazi_view_medium")),
        Some((" 3/f ", "yazi_view_full")),
        Some((" 4/w ", "yazi_view_wide")),
    ],
    [
        Some((" 5/d ", "yazi_view_detailed")),
        Some((" 6/x ", "yazi_view_descriptions")),
        Some((" 7/o ", "yazi_view_owners")),
        Some((" 8/l ", "yazi_view_links")),
    ],
    [
        Some((" 9/a ", "yazi_view_alt_full")),
        Some((" i ", "yazi_view_info")),
        Some((" q ", "yazi_view_quick")),
        None,
    ],
];
