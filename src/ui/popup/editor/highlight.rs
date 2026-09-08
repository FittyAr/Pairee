use ratatui::style::Style;
use ratatui::text::Span;

pub fn highlight_line(
    line: &str,
    query: &str,
    case_sensitive: bool,
    normal_style: Style,
    highlight_style: Style,
) -> Vec<Span<'static>> {
    if query.is_empty() {
        return vec![Span::styled(line.to_string(), normal_style)];
    }

    let mut spans = Vec::new();
    let query_len = query.chars().count();
    let line_chars: Vec<char> = line.chars().collect();
    let line_len = line_chars.len();

    let query_lower: Vec<char> = if case_sensitive {
        query.chars().collect()
    } else {
        query.to_lowercase().chars().collect()
    };

    let line_lower: Vec<char> = if case_sensitive {
        line_chars.clone()
    } else {
        line.to_lowercase().chars().collect()
    };

    let mut i = 0;
    while i < line_len {
        let mut matches = false;
        if i + query_len <= line_lower.len() {
            matches = true;
            for j in 0..query_len {
                if line_lower[i + j] != query_lower[j] {
                    matches = false;
                    break;
                }
            }
        }

        if matches {
            let match_str: String = line_chars[i..i + query_len].iter().collect();
            spans.push(Span::styled(match_str, highlight_style));
            i += query_len;
        } else {
            let mut normal_str = String::new();
            normal_str.push(line_chars[i]);
            i += 1;

            while i < line_len {
                let mut sub_matches = false;
                if i + query_len <= line_lower.len() {
                    sub_matches = true;
                    for j in 0..query_len {
                        if line_lower[i + j] != query_lower[j] {
                            sub_matches = false;
                            break;
                        }
                    }
                }
                if sub_matches {
                    break;
                }
                normal_str.push(line_chars[i]);
                i += 1;
            }
            spans.push(Span::styled(normal_str, normal_style));
        }
    }

    spans
}
