use crate::app::text_input;

/// Searches for the next occurrence of `query` in the editor.
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

    let pat_lower: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    // Returns a byte offset into `text` itself. Lowercasing the whole line
    // first would yield offsets into a different string (some characters
    // change byte length when lowercased), so compare char by char instead.
    let match_fn = |text: &str, pat: &str| -> Option<usize> {
        if case_sensitive {
            return text.find(pat);
        }
        text.char_indices().map(|(i, _)| i).find(|&i| {
            let mut hay = text[i..].chars().flat_map(char::to_lowercase);
            pat_lower.iter().all(|pc| hay.next() == Some(*pc))
        })
    };

    // 1. Search current line forward (starting at current_x + 1)
    if current_y < lines.len() {
        let line = &lines[current_y];
        let start_idx = text_input::next_boundary(line, current_x);
        if start_idx < line.len()
            && let Some(pos) = match_fn(&line[start_idx..], query)
        {
            return Some((start_idx + pos, current_y));
        }
    }

    // 2. Search subsequent lines forward
    for (y, line) in lines.iter().enumerate().skip(current_y + 1) {
        if let Some(pos) = match_fn(line, query) {
            return Some((pos, y));
        }
    }

    // 3. Wrap around: Search from start of file up to current_y
    for (y, line) in lines.iter().enumerate().take(current_y + 1) {
        let limit = if y == current_y {
            text_input::floor_boundary(line, current_x)
        } else {
            line.len()
        };
        if let Some(pos) = match_fn(&line[..limit], query) {
            return Some((pos, y));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_next_in_editor_non_ascii() {
        let lines = vec!["ñandú Ñandú".to_string(), "ÁRBOL árbol".to_string()];
        // Cursor on the multi-byte 'ñ' must not panic when searching forward.
        assert_eq!(
            find_next_in_editor(&lines, 0, 0, "ñandú", false),
            Some((8, 0))
        );
        assert_eq!(
            find_next_in_editor(&lines, 8, 0, "árbol", false),
            Some((0, 1))
        );
        assert_eq!(
            find_next_in_editor(&lines, 0, 1, "árbol", true),
            Some((7, 1))
        );
        // Stale mid-character cursor is tolerated.
        assert!(find_next_in_editor(&lines, 1, 0, "x", false).is_none());
    }

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
}
