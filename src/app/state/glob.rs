/// Matches `name` against a shell-style glob pattern supporting `*`, `?`, and `{a,b}` brace expansion.
pub fn glob_matches(pattern: &str, name: &str) -> bool {
    glob_matches_case(pattern, name, false)
}

/// Matches `name` against a shell-style glob pattern supporting case-sensitivity option.
pub fn glob_matches_case(pattern: &str, name: &str, case_sensitive: bool) -> bool {
    for pat in expand_braces(pattern) {
        if glob_match_iter(&pat, name, case_sensitive) {
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
            // If the brace group is empty (`{}`), or a single option with
            // no nested braces remains, keep the literal so we do not loop.
            if options.is_empty() {
                let mut s = String::with_capacity(pre.len() + post.len());
                s.push_str(pre);
                s.push_str(post);
                return vec![s];
            }
            let mut results = Vec::new();
            for opt in options.split(',') {
                // `opt` itself is allowed to contain nested braces; recurse
                // so the caller's upper bound on output size is still the
                // product of every alternative count, but each recursion
                // strictly reduces the remaining brace depth.
                let expanded = format!("{}{}{}", pre, opt, post);
                results.extend(expand_braces(&expanded));
            }
            return results;
        }
    }
    vec![pattern.to_string()]
}

/// Iterative glob matcher. Avoids the exponential backtracking blow-up
/// that a naive recursive implementation suffers from when the pattern
/// contains multiple `*` tokens followed by a character that the text does
/// not contain (e.g. matching `*a*a*a*a*a*a*b` against `aaaaaac`).
///
/// The algorithm walks the pattern and text in lock-step, saving a
/// back-track position whenever a `*` is encountered. If a later mismatch
/// occurs, it rewinds to the saved position and advances the text by one.
/// This is O(n*m) in the worst case but linear in practice.
fn glob_match_iter(pattern: &str, text: &str, case_sensitive: bool) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    let mut p_idx = 0usize;
    let mut t_idx = 0usize;
    // After a `*`, the next pattern position and the corresponding text
    // position are saved here so we can back-track on mismatch.
    let mut star_p: Option<usize> = None;
    let mut star_t: usize = 0;

    loop {
        if p_idx < pat.len() && pat[p_idx] == '*' {
            // Record the position after the `*` and the current text
            // position so we can back-track later.
            p_idx += 1;
            star_p = Some(p_idx);
            star_t = t_idx;
            continue;
        }
        if p_idx == pat.len() {
            // Pattern exhausted. We accept only if the text is also
            // exhausted, or if the remaining pattern is all `*`s (handled
            // implicitly by the `continue` branch on a `*`).
            if t_idx == txt.len() {
                return true;
            }
        } else {
            let p = pat[p_idx];
            if t_idx < txt.len() {
                let t = txt[t_idx];
                let matches = if case_sensitive {
                    p == t
                } else {
                    p.to_ascii_lowercase() == t.to_ascii_lowercase()
                };
                if matches || p == '?' {
                    p_idx += 1;
                    t_idx += 1;
                    continue;
                }
            }
        }
        // Mismatch (or pattern ended with text remaining). Back-track to
        // the most recent `*` if any, advancing the text by one.
        match star_p {
            Some(sp) if star_t < txt.len() => {
                p_idx = sp;
                t_idx = star_t + 1;
                star_t = t_idx;
            }
            _ => return false,
        }
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

    #[test]
    fn test_glob_brace_expansion() {
        assert!(glob_matches("*.{rs,toml}", "main.rs"));
        assert!(glob_matches("*.{rs,toml}", "main.toml"));
        assert!(!glob_matches("*.{rs,toml}", "main.md"));
    }

    #[test]
    fn test_glob_pathological_no_explosion() {
        // A naive recursive matcher blows up on this kind of input. The
        // iterative version must return in well under a second.
        let pattern = "*a*a*a*a*a*a*a*a*a*a*b";
        let text: String = std::iter::repeat('a').take(30).collect();
        let start = std::time::Instant::now();
        let matched = glob_matches(pattern, &text);
        let elapsed = start.elapsed();
        assert!(!matched);
        // The exact bound is loose, but the iterative algorithm should
        // complete in microseconds, not seconds. A recursive backtracking
        // implementation would take much longer.
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "glob match took too long: {:?}",
            elapsed
        );
    }
}
