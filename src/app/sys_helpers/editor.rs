/// Searches for the next occurrence of `query` in the editor.
///
/// `current_x` / `current_y` are **character** indices (the editor
/// exposes cursor position in chars, not bytes). The pre-fix
/// version did `&line[current_x + 1..]` directly, which
/// panicked with `"byte index N is not a char boundary"` the
/// moment the user opened a file containing accented characters
/// or any other non-ASCII text.
///
/// We now operate exclusively in char-index space and only
/// produce char-index results, so the slicing can never land in
/// the middle of a UTF-8 codepoint. The match itself still runs
/// on the underlying byte string (so a needle like "café"
/// matches the byte sequence, not the conceptual character
/// sequence), and the byte index of the match is converted back
/// to a char index before returning.
pub fn find_next_in_editor(
    lines: &[String],
    current_x: usize,
    current_y: usize,
    query: &str,
    case_sensitive: bool,
) -> Option<(usize, usize)> {
    if query.is_empty() || lines.is_empty() {
        return None;
    }

    let match_fn = |text: &str, pat: &str| -> Option<usize> {
        if case_sensitive {
            text.find(pat)
        } else {
            text.to_lowercase().find(&pat.to_lowercase())
        }
    };

    // Char-index slicing helper: returns the substring of `line`
    // starting at `start_chars` (inclusive) and ending at
    // `end_chars` (exclusive). Operates entirely in char space
    // so it can never cut a multi-byte codepoint in half. The
    // returned tuple `(slice, start_byte)` exposes the byte
    // offset of the slice start within the original line, so a
    // match byte offset within the slice can be promoted to a
    // char offset in the original line.
    //
    // We materialise the start byte of every char in a Vec so
    // we can look up both boundaries in O(1) by char index.
    // `byte_starts[i]` is the start byte of the `i`-th char,
    // and we append a sentinel `line.len()` at the end so that
    // `byte_starts[len_chars]` is the byte right after the last
    // char (the natural "end" for a slice that runs to the end
    // of the string).
    fn char_slice<'a>(line: &'a str, start_chars: usize, end_chars: usize) -> (&'a str, usize) {
        let len_chars = line.chars().count();
        let s = start_chars.min(len_chars);
        let e = end_chars.min(len_chars).max(s);
        let mut byte_starts: Vec<usize> = line.char_indices().map(|(b, _)| b).collect();
        byte_starts.push(line.len());
        // `byte_starts` now has `len_chars + 1` elements:
        // index 0 is the start of the first char, index
        // `len_chars` is the sentinel `line.len()`.
        let start_byte = byte_starts[s];
        let end_byte = byte_starts[e];
        (&line[start_byte..end_byte], start_byte)
    }

    // Convert a byte offset within `line` to a char offset.
    fn byte_to_char(line: &str, byte_off: usize) -> usize {
        line[..byte_off].chars().count()
    }

    // 1. Search current line forward (starting at current_x + 1)
    if current_y < lines.len() {
        let line = &lines[current_y];
        let total_chars = line.chars().count();
        let (slice, start_byte) = char_slice(line, current_x + 1, total_chars);
        if let Some(pos) = match_fn(slice, query) {
            let abs_byte = start_byte + pos;
            return Some((byte_to_char(line, abs_byte), current_y));
        }
    }

    // 2. Search subsequent lines forward
    for y in (current_y + 1)..lines.len() {
        let line = &lines[y];
        if let Some(pos) = match_fn(line, query) {
            return Some((byte_to_char(line, pos), y));
        }
    }

    // 3. Wrap around: Search from start of file up to current_y
    for y in 0..=current_y {
        let line = &lines[y];
        let end_chars = if y == current_y {
            current_x
        } else {
            line.chars().count()
        };
        let (slice, _start_byte) = char_slice(line, 0, end_chars);
        if let Some(pos) = match_fn(slice, query) {
            // The slice starts at char 0 of `line`, so a match
            // byte offset within the slice IS a byte offset
            // within the line.
            return Some((byte_to_char(line, pos), y));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_next_in_editor() {
        let lines = vec![
            "The quick brown fox".to_string(),
            "jumps over the lazy dog".to_string(),
            "The end".to_string(),
        ];

        // Case insensitive search
        assert_eq!(
            find_next_in_editor(&lines, 0, 0, "the", false),
            Some((11, 1))
        );
        assert_eq!(
            find_next_in_editor(&lines, 11, 1, "the", false),
            Some((0, 2))
        );
        assert_eq!(
            find_next_in_editor(&lines, 0, 2, "the", false),
            Some((0, 0))
        ); // Wrap around

        // Case sensitive search
        assert_eq!(find_next_in_editor(&lines, 0, 0, "The", true), Some((0, 2)));
        assert_eq!(
            find_next_in_editor(&lines, 0, 0, "the", true),
            Some((11, 1))
        );
        assert_eq!(
            find_next_in_editor(&lines, 11, 1, "The", true),
            Some((0, 2))
        );
        assert_eq!(find_next_in_editor(&lines, 0, 2, "The", true), Some((0, 0))); // Wrap around
    }

    /// Regression test: opening a file with multi-byte UTF-8
    /// (accents, CJK, emoji) and using the search must not panic
    /// with `"byte index is not a char boundary"`. Pre-fix, this
    /// would panic as soon as the user pressed F3 on a line
    /// containing `café` while the cursor was inside the word.
    #[test]
    fn test_find_next_in_editor_handles_multibyte_utf8() {
        let lines = vec![
            "línea uno: café".to_string(),
            "otra línea con ñ".to_string(),
            // 2-byte chars (latin), then a 3-byte char, then a
            // 4-byte char to exercise every boundary.
            "ascii 中文 😀 end".to_string(),
        ];

        // Search for "café" (4 chars, 5 bytes: c-a-f-é)
        // The match starts at char index 11 (after "línea uno: ").
        assert_eq!(
            find_next_in_editor(&lines, 0, 0, "café", false),
            Some((11, 0))
        );

        // Cursor past the end of the line; the function wraps
        // around and finds the match again on line 0. The
        // important property for this test is that it does
        // NOT panic on a multi-byte char at the cursor
        // position.
        assert_eq!(
            find_next_in_editor(&lines, 15, 0, "café", false),
            Some((11, 0))
        );

        // Cursor on a multi-byte char itself ('é' at char 14)
        // must not panic; the wrap-around search just doesn't
        // find the match in this line (the slice ends right
        // before the 'é').
        assert_eq!(find_next_in_editor(&lines, 14, 0, "café", false), None);

        // Searching for a non-existent query returns None cleanly
        // even when the line has multi-byte chars.
        assert_eq!(find_next_in_editor(&lines, 0, 0, "xyzzy", false), None);

        // Search across an emoji — the match must use char-index
        // positions, not byte positions.
        // "ascii 中文 😀 end"
        //  01234 5 (space is 1 char) 中 (1) 文 (1) (space) 😀 (1) ...
        // Char positions: "ascii" = 0..5, " " = 5, "中" = 6,
        // "文" = 7, " " = 8, "😀" = 9, " " = 10, "end" = 11..14.
        assert_eq!(
            find_next_in_editor(&lines, 0, 2, "end", false),
            Some((11, 2))
        );

        // Searching for the emoji itself
        assert_eq!(find_next_in_editor(&lines, 0, 2, "😀", false), Some((9, 2)));
    }
}
