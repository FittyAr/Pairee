//! Type-ahead find (`typing = "type_ahead"`): typing letters in a panel
//! jumps to the first entry whose name starts with them, as in Explorer and
//! Total Commander.

use super::panel_find::{Direction, NameMatch, find};
use std::time::{Duration, Instant};

/// Pause after which typing starts a new search.
const RESET_AFTER: Duration = Duration::from_millis(1000);

/// The letters typed so far.
#[derive(Debug, Default)]
pub struct TypeAheadBuffer {
    query: String,
    last: Option<Instant>,
}

impl TypeAheadBuffer {
    /// Adds `c` (starting over after a pause) and returns the query.
    pub fn push(&mut self, c: char, now: Instant) -> &str {
        if self
            .last
            .is_none_or(|t| now.saturating_duration_since(t) > RESET_AFTER)
        {
            self.query.clear();
        }
        self.query.extend(c.to_lowercase());
        self.last = Some(now);
        &self.query
    }
}

/// The entry to jump to for `query` from `cursor`: the first name with that
/// prefix at or after the cursor, wrapping. Repeating one letter (`"aaa"`)
/// cycles through the names starting with it, starting after the cursor.
pub fn find_match(names: &[&str], cursor: usize, query: &str) -> Option<usize> {
    let first = query.chars().next()?;
    let (needle, start) = if query.chars().all(|c| c == first) {
        (first.to_string(), cursor + 1)
    } else {
        (query.to_string(), cursor)
    };
    find(names, start, &needle, NameMatch::Prefix, Direction::Forward)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: &[&str] = &["..", "Apple", "apricot", "Banana", "berry", "cherry"];

    fn find(cursor: usize, query: &str) -> Option<usize> {
        find_match(NAMES, cursor, query)
    }

    #[test]
    fn prefix_jumps_case_insensitively() {
        assert_eq!(find(0, "b"), Some(3));
        assert_eq!(find(0, "apr"), Some(2));
        assert_eq!(find(5, "ch"), Some(5));
        assert_eq!(find(0, "zz"), None);
    }

    #[test]
    fn repeating_a_letter_cycles_and_wraps() {
        assert_eq!(find(1, "a"), Some(2));
        assert_eq!(find(2, "aa"), Some(1));
        assert_eq!(find(3, "b"), Some(4));
    }

    #[test]
    fn a_pause_starts_a_new_query() {
        let mut buf = TypeAheadBuffer::default();
        let t0 = Instant::now();
        assert_eq!(buf.push('A', t0), "a");
        assert_eq!(buf.push('p', t0 + Duration::from_millis(300)), "ap");
        assert_eq!(buf.push('b', t0 + Duration::from_secs(3)), "b");
    }
}
