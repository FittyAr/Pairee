use super::KeyGrid;

/// Sort keys of the Yazi-style popup, three rows of four.
pub const SORT_KEYS: KeyGrid = [
    [
        Some((" n ", "yazi_sort_name")),
        Some((" e ", "yazi_sort_extension")),
        Some((" s ", "yazi_sort_size")),
        Some((" w ", "yazi_sort_write")),
    ],
    [
        Some((" c ", "yazi_sort_creation")),
        Some((" a ", "yazi_sort_access")),
        Some((" d ", "yazi_sort_description")),
        Some((" o ", "yazi_sort_owner")),
    ],
    [
        Some((" u ", "yazi_sort_unsorted")),
        Some((" r ", "yazi_sort_reverse")),
        None,
        None,
    ],
];
