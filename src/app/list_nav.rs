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

    /// Like [`Self::apply`] but Up / Down stop at the ends instead of
    /// wrapping (long lists such as the Git panel tabs).
    pub fn apply_clamped(self, idx: usize, len: usize) -> usize {
        match self {
            Self::Prev => idx.saturating_sub(1),
            Self::Next if len > 0 => (idx + 1).min(len - 1),
            Self::Next => idx,
            _ => self.apply(idx, len),
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

/// Which navigation keys a list popup accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListKeys {
    /// `k` / `j` act like Up / Down.
    pub vim: bool,
    /// PgUp / PgDn / Home / End are accepted.
    pub paging: bool,
}

impl ListKeys {
    pub const ARROWS: Self = Self {
        vim: false,
        paging: false,
    };
    pub const ARROWS_VIM: Self = Self {
        vim: true,
        paging: false,
    };
    pub const FULL: Self = Self {
        vim: false,
        paging: true,
    };
}

/// Meaning of a key in a selectable list popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKey {
    /// A navigation key (the cursor moved unless the list is empty).
    Moved,
    /// Enter on row `n` (may be out of range for an empty list).
    Activate(usize),
    /// Esc.
    Close,
    /// Anything else, left to the popup.
    Other,
}

/// Applies the navigation keys allowed by `keys` to `cursor` and classifies
/// Enter / Esc, so list popups only handle their own actions.
pub fn list_key(keys: ListKeys, code: KeyCode, cursor: &mut usize, len: usize) -> ListKey {
    let code = match code {
        KeyCode::Char('k' | 'K') if keys.vim => KeyCode::Up,
        KeyCode::Char('j' | 'J') if keys.vim => KeyCode::Down,
        other => other,
    };
    let step = if keys.paging {
        NavStep::from_key(code)
    } else {
        NavStep::from_arrow(code)
    };
    match code {
        _ if step.is_some() => {
            if let Some(step) = step {
                *cursor = step.apply(*cursor, len);
            }
            ListKey::Moved
        }
        KeyCode::Enter => ListKey::Activate(*cursor),
        KeyCode::Esc => ListKey::Close,
        _ => ListKey::Other,
    }
}

/// Keys accepted by a scrolled read-only text (About, Help, diffs...).
#[derive(Debug, Clone, Copy)]
pub struct ScrollKeys {
    /// `k` / `j` scroll like Up / Down.
    pub vim: bool,
    /// Home / End jump to the top / bottom.
    pub home_end: bool,
    /// Lines moved by PgUp / PgDn.
    pub page: usize,
}

/// Applies a scroll key to `scroll`, never past `last`. Returns `false` for
/// other keys.
pub fn scroll_key(keys: ScrollKeys, code: KeyCode, scroll: &mut usize, last: usize) -> bool {
    let next = match code {
        KeyCode::Up => scroll.saturating_sub(1),
        KeyCode::Char('k' | 'K') if keys.vim => scroll.saturating_sub(1),
        KeyCode::Down => scroll.saturating_add(1),
        KeyCode::Char('j' | 'J') if keys.vim => scroll.saturating_add(1),
        KeyCode::PageUp => scroll.saturating_sub(keys.page),
        KeyCode::PageDown => scroll.saturating_add(keys.page),
        KeyCode::Home if keys.home_end => 0,
        KeyCode::End if keys.home_end => last,
        _ => return false,
    };
    *scroll = next.min(last);
    true
}

/// Meaning of a key in a type-to-filter list (command palette, which-key).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterKey {
    /// Cursor moved (or nothing happened).
    Moved,
    /// The query text changed: refilter and reset the cursor.
    QueryChanged,
    /// Enter on visible row `n`.
    Activate(usize),
    /// Esc.
    Close,
}

/// Up / Down move (without wrapping) inside `visible` rows, Enter / Esc
/// activate / close, anything else edits `query`.
pub fn filter_list_key(
    query: &mut crate::app::text_input::TextField,
    cursor: &mut usize,
    visible: usize,
    key: &crossterm::event::KeyEvent,
) -> FilterKey {
    match key.code {
        KeyCode::Up => *cursor = cursor.saturating_sub(1).min(visible.saturating_sub(1)),
        KeyCode::Down => *cursor = (*cursor + 1).min(visible.saturating_sub(1)),
        KeyCode::Enter => return FilterKey::Activate((*cursor).min(visible.saturating_sub(1))),
        KeyCode::Esc => return FilterKey::Close,
        _ => {
            if query.handle_edit_key(key) == crate::app::text_input::FieldEdit::Edited {
                *cursor = 0;
                return FilterKey::QueryChanged;
            }
        }
    }
    FilterKey::Moved
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
        assert_eq!(
            list_key(ListKeys::FULL, KeyCode::Down, &mut cursor, 0),
            ListKey::Moved
        );
        assert_eq!(cursor, 0);
    }

    #[test]
    fn list_key_maps_vim_paging_enter_and_esc() {
        let mut cursor = 0;
        assert_eq!(
            list_key(ListKeys::ARROWS_VIM, KeyCode::Char('k'), &mut cursor, 3),
            ListKey::Moved
        );
        assert_eq!(cursor, 2);
        assert_eq!(
            list_key(ListKeys::ARROWS, KeyCode::Char('k'), &mut cursor, 3),
            ListKey::Other
        );
        assert_eq!(
            list_key(ListKeys::ARROWS, KeyCode::Home, &mut cursor, 3),
            ListKey::Other
        );
        assert_eq!(
            list_key(ListKeys::FULL, KeyCode::Home, &mut cursor, 3),
            ListKey::Moved
        );
        assert_eq!(cursor, 0);
        assert_eq!(
            list_key(ListKeys::FULL, KeyCode::Enter, &mut cursor, 3),
            ListKey::Activate(0)
        );
        assert_eq!(
            list_key(ListKeys::FULL, KeyCode::Esc, &mut cursor, 3),
            ListKey::Close
        );
        assert_eq!(
            list_key(ListKeys::FULL, KeyCode::Down, &mut cursor, 0),
            ListKey::Moved
        );
    }

    #[test]
    fn stale_cursor_past_the_end_wraps_to_last() {
        assert_eq!(wrap_prev(9, 3), 2);
        assert_eq!(wrap_next(9, 3), 0);
    }
}
