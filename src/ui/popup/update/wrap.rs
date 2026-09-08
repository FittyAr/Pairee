use ratatui::text::{Line, Span};

// Simple word-wrapping helper
pub fn wrap_lines(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    let mut wrapped = Vec::new();
    for line in lines {
        let total_chars: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
        if total_chars <= width {
            wrapped.push(line);
            continue;
        }

        let mut current_line_spans = Vec::new();
        let mut current_width = 0;

        for span in line.spans {
            let text = span.content.into_owned();
            let style = span.style;

            let mut words = Vec::new();
            let mut word = String::new();
            for c in text.chars() {
                if c.is_whitespace() {
                    if !word.is_empty() {
                        words.push((word.clone(), false));
                        word.clear();
                    }
                    words.push((c.to_string(), true));
                } else {
                    word.push(c);
                }
            }
            if !word.is_empty() {
                words.push((word, false));
            }

            for (w, is_space) in words {
                let w_len = w.chars().count();
                if current_width + w_len > width && !is_space && current_width > 0 {
                    wrapped.push(Line::from(current_line_spans));
                    current_line_spans = Vec::new();
                    current_width = 0;
                }

                if w_len > width {
                    let chars: Vec<char> = w.chars().collect();
                    for chunk in chars.chunks(width) {
                        let chunk_str: String = chunk.iter().collect();
                        wrapped.push(Line::from(vec![Span::styled(chunk_str, style)]));
                    }
                    continue;
                }

                current_line_spans.push(Span::styled(w, style));
                current_width += w_len;
            }
        }
        if !current_line_spans.is_empty() {
            wrapped.push(Line::from(current_line_spans));
        }
    }
    wrapped
}
