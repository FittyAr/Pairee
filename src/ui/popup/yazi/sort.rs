use super::KeyGrid;

/// Sort keys of the Yazi-style popup, three rows of four.
pub const SORT_KEYS: KeyGrid = [
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
