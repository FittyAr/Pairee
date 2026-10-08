//! Selection-cursor movement shared by every list-style popup (menus, history
//! lists, hotlist, drive selector, plugin lists, Git tabs, dialog rows...).
//!
//! Handlers translate a key into a [`NavStep`] with [`NavStep::from_key`] and
//! apply it to their cursor; the arithmetic (wrapping, paging, clamping) lives
//! here once.

use crossterm::event::KeyCode;

/// Rows skipped by PgUp / PgDn in popup lists.
pub const PAGE_SIZE: usize = 10;

/// One cursor movement in a list of `len` rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavStep {
    /// Previous row, wrapping to the last one.
    Prev,
    /// Next row, wrapping to the first one.
    Next,
    /// [`PAGE_SIZE`] rows up, stopping at the first row.
    PageUp,
    /// [`PAGE_SIZE`] rows down, stopping at the last row.
    PageDown,
    /// First row.
    First,
    /// Last row.
    Last,
}

impl NavStep {
    /// Maps Up / Down / PgUp / PgDn / Home / End to a step.
    pub fn from_key(code: KeyCode) -> Option<Self> {
        Some(match code {
            KeyCode::Up => Self::Prev,
            KeyCode::Down => Self::Next,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,
            KeyCode::Home => Self::First,
            KeyCode::End => Self::Last,
            _ => return None,
        })
    }

    /// Maps only the wrapping Up / Down arrows (lists that use the other keys
    /// for something else).
    pub fn from_arrow(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::Up => Some(Self::Prev),
            KeyCode::Down => Some(Self::Next),
            _ => None,
        }
    }

    /// New cursor position for a list of `len` rows (`idx` unchanged when empty).
    pub fn apply(self, idx: usize, len: usize) -> usize {
        if len == 0 {
            return idx;
        }
        let last = len - 1;
        match self {
            Self::Prev => wrap_prev(idx, len),
            Self::Next => wrap_next(idx, len),
            Self::PageUp => idx.saturating_sub(PAGE_SIZE).min(last),
            Self::PageDown => idx.saturating_add(PAGE_SIZE).min(last),
            Self::First => 0,
            Self::Last => last,
        }
    }
}

/// Previous index in `0..len`, wrapping from the first to the last row.
pub fn wrap_prev(idx: usize, len: usize) -> usize {
    match len {
        0 => 0,
        _ if idx == 0 || idx >= len => len - 1,
        _ => idx - 1,
    }
}

/// Next index in `0..len`, wrapping from the last to the first row.
pub fn wrap_next(idx: usize, len: usize) -> usize {
    if idx + 1 >= len { 0 } else { idx + 1 }
}

/// Applies Up / Down / PgUp / PgDn / Home / End to `cursor` in a list of `len`
/// rows. Returns `true` when the key was a navigation key and the list is not
/// empty.
pub fn handle_list_nav(code: KeyCode, cursor: &mut usize, len: usize) -> bool {
    apply_step(NavStep::from_key(code), cursor, len)
}

/// Like [`handle_list_nav`] but only for the wrapping Up / Down arrows.
pub fn handle_arrow_nav(code: KeyCode, cursor: &mut usize, len: usize) -> bool {
    apply_step(NavStep::from_arrow(code), cursor, len)
}

fn apply_step(step: Option<NavStep>, cursor: &mut usize, len: usize) -> bool {
    match step {
        Some(step) if len > 0 => {
            *cursor = step.apply(*cursor, len);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_wrap_around() {
        assert_eq!(NavStep::Prev.apply(0, 3), 2);
        assert_eq!(NavStep::Prev.apply(2, 3), 1);
        assert_eq!(NavStep::Next.apply(2, 3), 0);
        assert_eq!(NavStep::Next.apply(0, 3), 1);
    }

    #[test]
    fn paging_clamps_to_bounds() {
        assert_eq!(NavStep::PageUp.apply(4, 30), 0);
        assert_eq!(NavStep::PageUp.apply(25, 30), 15);
        assert_eq!(NavStep::PageDown.apply(25, 30), 29);
        assert_eq!(NavStep::First.apply(7, 30), 0);
        assert_eq!(NavStep::Last.apply(7, 30), 29);
    }

    #[test]
    fn empty_list_is_not_navigated() {
        let mut cursor = 0;
        assert!(!handle_list_nav(KeyCode::Down, &mut cursor, 0));
        assert_eq!(cursor, 0);
    }

    #[test]
    fn arrow_nav_ignores_paging_keys() {
        let mut cursor = 1;
        assert!(!handle_arrow_nav(KeyCode::Home, &mut cursor, 5));
        assert!(handle_arrow_nav(KeyCode::Up, &mut cursor, 5));
        assert_eq!(cursor, 0);
    }

    #[test]
    fn stale_cursor_past_the_end_wraps_to_last() {
        assert_eq!(wrap_prev(9, 3), 2);
        assert_eq!(wrap_next(9, 3), 0);
    }
}
