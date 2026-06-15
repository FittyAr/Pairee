/// Matches `name` against a shell-style glob pattern supporting `*`, `?`, and `{a,b}` brace expansion.
pub fn glob_matches(pattern: &str, name: &str) -> bool {
    glob_matches_case(pattern, name, false)
}

/// Matches `name` against a shell-style glob pattern supporting case-sensitivity option.
pub fn glob_matches_case(pattern: &str, name: &str, case_sensitive: bool) -> bool {
    for pat in expand_braces(pattern) {
        if glob_match_inner(pat.as_bytes(), name.as_bytes(), case_sensitive) {
            return true;
        }
    }
    false
}

fn expand_braces(pattern: &str) -> Vec<String> {
    if let Some(start) = pattern.find('{') {
        if let Some(end) = pattern[start..].find('}') {
            let end = start + end;
            let pre = &pattern[..start];
            let post = &pattern[end + 1..];
            let options = &pattern[start + 1..end];
            let mut results = Vec::new();
            for opt in options.split(',') {
                let expanded = format!("{}{}{}", pre, opt, post);
                results.extend(expand_braces(&expanded));
            }
            return results;
        }
    }
    vec![pattern.to_string()]
}

fn glob_match_inner(pat: &[u8], text: &[u8], case_sensitive: bool) -> bool {
    match (pat.first(), text.first()) {
        (None, None) => true,
        (Some(&b'*'), _) => {
            // Try consuming zero or more chars from text
            glob_match_inner(&pat[1..], text, case_sensitive)
                || (!text.is_empty() && glob_match_inner(pat, &text[1..], case_sensitive))
        }
        (Some(&b'?'), Some(_)) => glob_match_inner(&pat[1..], &text[1..], case_sensitive),
        (Some(p), Some(t)) => {
            let matches = if case_sensitive {
                p == t
            } else {
                p.to_ascii_lowercase() == t.to_ascii_lowercase()
            };
            matches && glob_match_inner(&pat[1..], &text[1..], case_sensitive)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_matches() {
        assert!(glob_matches("*.rs", "main.rs"));
        assert!(glob_matches("*.rs", "lib.rs"));
        assert!(!glob_matches("*.rs", "main.toml"));
        assert!(glob_matches("foo?ar", "foobar"));
        assert!(glob_matches("*", "anything"));
        assert!(!glob_matches("*.rs", ""));
    }
}
