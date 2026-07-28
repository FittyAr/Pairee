/// Matches `name` against a shell-style glob pattern supporting `*`, `?`, and `{a,b}` brace expansion.
///
/// # Examples
///
/// ```no_run
/// use crate::app::state::glob::glob_matches;
///
/// // `*` matches any string.
/// assert!(glob_matches("*.rs", "main.rs"));
/// assert!(!glob_matches("*.rs", "main.toml"));
/// // `?` matches a single character.
/// assert!(glob_matches("foo?ar", "foobar"));
/// // Brace expansion picks any of the alternatives.
/// assert!(glob_matches("*.{rs,toml}", "Cargo.toml"));
/// ```
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

    #[test]
    fn test_glob_question_mark() {
        // `?` matches exactly one character.
        assert!(glob_matches("a?c", "abc"));
        assert!(glob_matches("a?c", "axc"));
        assert!(!glob_matches("a?c", "ac")); // too short
        assert!(!glob_matches("a?c", "abbc")); // too long
        assert!(!glob_matches("?", "")); // requires one char
    }

    #[test]
    fn test_glob_star_matches_empty() {
        // `*` should match the empty string at any position.
        assert!(glob_matches("*", ""));
        assert!(glob_matches("a*", "a"));
        assert!(glob_matches("*b", "b"));
        assert!(glob_matches("a*b", "ab"));
    }

    #[test]
    fn test_glob_brace_expansion_matches_any_option() {
        assert!(glob_matches("*.{rs,toml}", "main.rs"));
        assert!(glob_matches("*.{rs,toml}", "Cargo.toml"));
        assert!(!glob_matches("*.{rs,toml}", "main.md"));
        assert!(glob_matches("{a,b,c}", "b"));
        assert!(!glob_matches("{a,b,c}", "d"));
    }

    #[test]
    fn test_glob_brace_nested_prefix_suffix() {
        // Brace expansion substitutes into the middle of a pattern.
        assert!(glob_matches("file_{one,two,three}.txt", "file_two.txt"));
        assert!(glob_matches("file_{one,two,three}.txt", "file_one.txt"));
        assert!(!glob_matches("file_{one,two,three}.txt", "file_four.txt"));
        assert!(glob_matches("pre{a,b}post", "preapost"));
        assert!(glob_matches("pre{a,b}post", "prebpost"));
        assert!(!glob_matches("pre{a,b}post", "prepost")); // requires one of a or b
    }

    #[test]
    fn test_glob_case_sensitivity_default_is_insensitive() {
        // glob_matches() is case insensitive by default.
        assert!(glob_matches("*.RS", "main.rs"));
        assert!(glob_matches("*.rs", "MAIN.RS"));
        assert!(glob_matches("README", "readme"));
    }

    #[test]
    fn test_glob_case_sensitive_strict() {
        // When explicitly case-sensitive, only exact matches.
        assert!(glob_matches_case("*.rs", "main.rs", true));
        assert!(!glob_matches_case("*.RS", "main.rs", true));
        assert!(glob_matches_case("README", "README", true));
        assert!(!glob_matches_case("README", "readme", true));
    }

    #[test]
    fn test_glob_only_literal_chars() {
        // No wildcards = exact equality required.
        assert!(glob_matches("exact", "exact"));
        assert!(!glob_matches("exact", "exactly"));
        assert!(!glob_matches("exact", "exac"));
    }

    #[test]
    fn test_glob_unbalanced_brace_is_treated_as_literal() {
        // `expand_braces` only matches well-formed `{...}` — an
        // unbalanced brace is left as-is in the single-element result.
        // This documents the current (lenient) behaviour.
        assert!(glob_matches("a{b", "a{b"));
        assert!(!glob_matches("a{b", "ab"));
    }

    #[test]
    fn test_glob_special_chars_are_literal() {
        // `.` and `/` and `\` are literal in the glob (no regex).
        assert!(glob_matches("a.b", "a.b"));
        assert!(!glob_matches("a.b", "aXb"));
        assert!(glob_matches("path/to", "path/to"));
        assert!(!glob_matches("path/to", "pathXto"));
    }

    // ── Property-based tests ──────────────────────────────────────────────────
    //
    // Property: a single `*` must match every possible name.
    // This catches any future change that adds an "empty name" edge case
    // to the matcher.
    #[test]
    fn prop_star_matches_everything() {
        use proptest::prelude::*;
        proptest!(|(name in ".*")| {
            prop_assert!(glob_matches("*", &name));
        });
    }

    // Property: case-insensitive and case-sensitive matchers must agree
    // when the inputs are already in the same case. They must differ
    // when only one side has uppercase letters.
    #[test]
    fn prop_case_sensitivity_is_consistent() {
        use proptest::prelude::*;
        proptest!(|(pat in "[a-zA-Z*?]+", name in "[a-zA-Z]+")| {
            let ci = glob_matches_case(&pat, &name, false);
            let cs = glob_matches_case(&pat, &name, true);
            // Lowercased pattern + lowercased name should match under
            // both modes if it matches under case-sensitive.
            if cs {
                prop_assert!(ci, "ci should accept anything cs accepts: pat={} name={}", pat, name);
            }
        });
    }

    // Property: literal patterns (no wildcards) must match exact strings
    // and only exact strings.
    #[test]
    fn prop_literal_match_is_equality() {
        use proptest::prelude::*;
        proptest!(|(s in "[a-zA-Z0-9]{1,20}")| {
            prop_assert!(glob_matches(&s, &s));
            prop_assert!(!glob_matches(&s, ""));
            let s2 = format!("{}x", s);
            prop_assert!(!glob_matches(&s, &s2));
        });
    }
}
