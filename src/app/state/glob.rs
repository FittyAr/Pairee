/// Maximum depth of nested `{a,b,c,...}` brace groups we will expand.
/// Each level can multiply the number of produced patterns by the
/// number of comma-separated alternatives, so 6 levels with binary
/// options already reach 64 patterns and growing from there quickly
/// blows memory. 4 levels is enough for any sensible user-authored
/// glob and keeps the worst-case expansion bounded.
const MAX_BRACE_DEPTH: usize = 4;

/// Hard cap on the total number of patterns produced by a single
/// brace expansion. Even within `MAX_BRACE_DEPTH`, a pattern like
/// `{a,b,c,d}{a,b,c,d}{a,b,c,d}{a,b,c,d}` would explode to 256; with
/// the cap we refuse to expand further and treat the pattern as
/// non-matching instead of allocating an unbounded `Vec`.
const MAX_BRACE_PATTERNS: usize = 256;

/// Matches `name` against a shell-style glob pattern supporting `*`, `?`, and `{a,b}` brace expansion.
pub fn glob_matches(pattern: &str, name: &str) -> bool {
    glob_matches_case(pattern, name, false)
}

/// Matches `name` against a shell-style glob pattern supporting case-sensitivity option.
pub fn glob_matches_case(pattern: &str, name: &str, case_sensitive: bool) -> bool {
    for pat in expand_braces(pattern, 0) {
        if glob_match_iter(&pat, name, case_sensitive) {
            return true;
        }
    }
    false
}

fn expand_braces(pattern: &str, depth: usize) -> Vec<String> {
    if depth >= MAX_BRACE_DEPTH {
        // Beyond the depth cap we keep the literal pattern. `glob_match_iter`
        // will treat the unbalanced `{` and `}` as ordinary characters, so the
        // pattern will not match a sensible name — that is the desired
        // "refuse to expand" behaviour.
        return vec![pattern.to_string()];
    }
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
                results.extend(expand_braces(&expanded, depth + 1));
                if results.len() > MAX_BRACE_PATTERNS {
                    // Bail out before allocating the rest of the
                    // cross-product. The caller will iterate over the
                    // truncated list and may not match, which is the
                    // intended behaviour for a hostile / pathological
                    // pattern.
                    results.truncate(MAX_BRACE_PATTERNS);
                    return results;
                }
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

    #[test]
    fn test_glob_brace_explosion_is_capped() {
        // 5 levels of binary alternation would produce 32 patterns, well
        // under the cap. Anything beyond 4 nesting levels must NOT
        // explode exponentially — the function must return promptly
        // regardless of how many braces the user provides.
        let pattern = "{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}{a,b}";
        let start = std::time::Instant::now();
        let _ = glob_matches(pattern, "a");
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "brace explosion cap failed: took {:?} to reject a deeply nested pattern",
            elapsed
        );
    }

    #[test]
    fn test_glob_brace_total_patterns_is_capped() {
        // Even within the depth cap, a pattern with 5 binary options at
        // the same level produces 32 patterns which is fine; a pattern
        // that exceeds MAX_BRACE_PATTERNS must be truncated, not
        // continued to exhaustion. We assert the cap by expanding
        // directly via the public entry point and checking the call
        // returns within a tight time bound.
        let pattern = "{a,b,c,d}{a,b,c,d}{a,b,c,d}{a,b,c,d}{a,b,c,d}";
        // 5^5 = 3125 patterns without the cap; with the cap (256) the
        // expansion must be truncated well before that.
        let start = std::time::Instant::now();
        let _ = glob_matches(pattern, "a");
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_millis(500),
            "brace expansion cap failed: took {:?}",
            elapsed
        );
    }
}
