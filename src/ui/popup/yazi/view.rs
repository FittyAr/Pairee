use super::KeyGrid;

/// View-mode keys of the Yazi-style popup, three rows of four.
pub const VIEW_KEYS: KeyGrid = [
    [
        Some((" 1/b ", " Brief")),
        Some((" 2/m ", " Medium")),
        Some((" 3/f ", " Full")),
        Some((" 4/w ", " Wide")),
    ],
    [
        Some((" 5/d ", " Detailed")),
        Some((" 6/x ", " Descriptions")),
        Some((" 7/o ", " File owners")),
        Some((" 8/l ", " File links")),
    ],
    [
        Some((" 9/a ", " Alt full")),
        Some((" i ", " Info panel")),
        Some((" q ", " Quick view")),
        None,
    ],
];
