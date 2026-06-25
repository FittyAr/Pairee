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

    let match_fn = |text: &str, pat: &str| -> Option<usize> {
        if case_sensitive {
            text.find(pat)
        } else {
            text.to_lowercase().find(&pat.to_lowercase())
        }
    };

    // 1. Search current line forward (starting at current_x + 1)
    if current_y < lines.len() {
        let line = &lines[current_y];
        let start_idx = current_x + 1;
        if start_idx < line.len() {
            if let Some(pos) = match_fn(&line[start_idx..], query) {
                return Some((start_idx + pos, current_y));
            }
        }
    }

    // 2. Search subsequent lines forward
    for y in (current_y + 1)..lines.len() {
        if let Some(pos) = match_fn(&lines[y], query) {
            return Some((pos, y));
        }
    }

    // 3. Wrap around: Search from start of file up to current_y
    for y in 0..=current_y {
        let line = &lines[y];
        let limit = if y == current_y {
            current_x
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
