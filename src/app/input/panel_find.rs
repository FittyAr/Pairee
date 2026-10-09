//! Finding panel entries by name. Type-ahead, the in-panel search (`/`,
//! `n`, `N`) and Norton Commander's Alt+letter quick search share it.

/// How a name matches the query (always ignoring case).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameMatch {
    /// The name starts with the query (type-ahead, NC quick search).
    Prefix,
    /// The name contains the query (Vim / yazi `/`).
    Substring,
}

impl NameMatch {
    fn hits(self, name: &str, needle: &str) -> bool {
        let name = name.to_lowercase();
        match self {
            Self::Prefix => name.starts_with(needle),
            Self::Substring => name.contains(needle),
        }
    }
}

/// Which way to look from the start index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

/// The first index from `start` (included) going `direction`, wrapping
/// around, whose name matches `query`.
pub fn find(
    names: &[&str],
    start: usize,
    query: &str,
    how: NameMatch,
    direction: Direction,
) -> Option<usize> {
    let n = names.len();
    if n == 0 || query.is_empty() {
        return None;
    }
    let needle = query.to_lowercase();
    let start = start % n;
    (0..n)
        .map(|k| match direction {
            Direction::Forward => (start + k) % n,
            Direction::Backward => (start + n - k) % n,
        })
        .find(|&i| how.hits(names[i], &needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: &[&str] = &["..", "Apple", "pineapple", "Banana", "grape"];

    #[test]
    fn prefix_and_substring_wrap_both_ways() {
        use Direction::*;
        use NameMatch::*;
        assert_eq!(find(NAMES, 0, "app", Prefix, Forward), Some(1));
        assert_eq!(find(NAMES, 2, "app", Prefix, Forward), Some(1));
        assert_eq!(find(NAMES, 2, "APP", Substring, Forward), Some(2));
        assert_eq!(find(NAMES, 3, "ap", Substring, Forward), Some(4));
        assert_eq!(find(NAMES, 0, "ap", Substring, Backward), Some(4));
        assert_eq!(
            find(NAMES, 5, "an", Substring, Forward),
            Some(3),
            "start wraps"
        );
        assert_eq!(find(NAMES, 0, "zzz", Substring, Forward), None);
        assert_eq!(find(&[], 0, "a", Prefix, Forward), None);
    }
}
