//! Emulation of `cmd.exe` command-line `%VAR%` expansion.
//!
//! Used when a script has to reach `cmd.exe` through an environment variable
//! (see [`super::shell_invocation`]): cmd expands that variable once and does
//! not re-expand its contents, so the user's own `%VAR%` references are
//! expanded here first, with the same rules cmd applies on a command line:
//! a defined name is replaced; an undefined one is kept literally and the
//! closing `%` may start the next reference. Substring / replace syntax
//! (`%VAR:~0,3%`) is not supported and stays literal.

/// Expand `%NAME%` references in `script` using `lookup`.
pub fn expand_percent_vars(script: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(script.len());
    let mut rest = script;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                match (!name.is_empty()).then(|| lookup(name)).flatten() {
                    Some(value) => {
                        out.push_str(&value);
                        rest = &after[end + 1..];
                    }
                    None => {
                        out.push('%');
                        out.push_str(name);
                        rest = &after[end..];
                    }
                }
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(name: &str) -> Option<String> {
        match name.to_ascii_uppercase().as_str() {
            "FOO" => Some("EXP".into()),
            "HOME" => Some(r"C:\Users\me".into()),
            _ => None,
        }
    }

    #[test]
    fn expands_defined_and_keeps_undefined() {
        assert_eq!(expand_percent_vars("echo %foo%", env), "echo EXP");
        assert_eq!(expand_percent_vars("%FOO%%FOO%", env), "EXPEXP");
        assert_eq!(expand_percent_vars("%undefined%FOO%", env), "%undefinedEXP");
        assert_eq!(expand_percent_vars("100%", env), "100%");
        assert_eq!(expand_percent_vars("%%", env), "%%");
        assert_eq!(
            expand_percent_vars("cd %HOME%\\x", env),
            r"cd C:\Users\me\x"
        );
    }

    #[test]
    fn caret_escaped_names_stay_literal() {
        let quoted = super::super::quote::quote_cmd("%FOO%.txt");
        assert_eq!(expand_percent_vars(&quoted, env), quoted);
    }
}
