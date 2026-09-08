pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }

        let char_count = line.chars().count();
        if char_count <= max_width {
            lines.push(line.to_string());
        } else {
            let mut current_line = String::new();
            let mut current_len = 0;
            for word in line.split(' ') {
                let word_len = word.chars().count();
                if current_line.is_empty() {
                    current_line = word.to_string();
                    current_len = word_len;
                } else if current_len + 1 + word_len <= max_width {
                    current_line.push(' ');
                    current_line.push_str(word);
                    current_len += 1 + word_len;
                } else {
                    lines.push(current_line);
                    current_line = word.to_string();
                    current_len = word_len;
                }

                while current_len > max_width {
                    let chars: Vec<char> = current_line.chars().collect();
                    let head: String = chars[..max_width].iter().collect();
                    let tail: String = chars[max_width..].iter().collect();
                    lines.push(head);
                    current_line = tail;
                    current_len = current_line.chars().count();
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
    }
    lines
}
