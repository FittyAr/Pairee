//! Quoting of untrusted values (file names, paths) embedded in a command
//! line that is later interpreted by `/bin/sh -c` or `cmd.exe /S /C`.

use std::path::Path;

/// POSIX sh: single-quote the whole token, escape any `'` as `'\''`
/// (close, literal quote, reopen). Every other metacharacter is inert inside
/// `'...'`.
pub fn quote_posix(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Characters `cmd.exe` interprets on a command line. Each one is emitted
/// as `^c`, so cmd never sees a quoted region, never expands `%VAR%` (the
/// name would end in `^`, which is never defined) and never splits on
/// `& | < > ( )`. `!` is covered for the case delayed expansion is on.
const CMD_METACHARS: &[char] = &['(', ')', '%', '!', '^', '"', '<', '>', '&', '|'];

/// Quote one argument for a program started by `cmd.exe /S /C`.
///
/// Two layers: first the MSVCRT argv rules used by the target program
/// (`"..."`, `\"` for quotes, backslashes before quotes doubled), then every
/// cmd metacharacter of that result is caret-escaped. cmd removes the carets
/// and hands the program exactly the MSVCRT-quoted argument, so the program
/// receives the original string as a single argv entry.
pub fn quote_cmd(s: &str) -> String {
    let argv = quote_msvcrt(s);
    let mut out = String::with_capacity(argv.len() * 2);
    for c in argv.chars() {
        if CMD_METACHARS.contains(&c) {
            out.push('^');
        }
        out.push(c);
    }
    out
}

/// MSVCRT / `CommandLineToArgvW` quoting (always wrapped in quotes).
fn quote_msvcrt(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for c in s.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(c);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

/// Quote for the platform shell used by [`super::shell_command`].
pub fn quote_native(s: &str) -> String {
    if cfg!(windows) {
        quote_cmd(s)
    } else {
        quote_posix(s)
    }
}

/// [`quote_native`] for a path.
pub fn quote_path(path: &Path) -> String {
    quote_native(&path.to_string_lossy())
}

/// Expand the user-menu placeholders `{f}` (file name) and `{p}` (full path)
/// in a single left-to-right pass, quoting each value with `quote`.
///
/// Values are never rescanned, so a file named `;id;{p}` cannot smuggle a
/// second placeholder whose expansion would land inside (and close) the
/// quotes of the first one.
pub fn expand_placeholders(
    template: &str,
    name: &str,
    path: &str,
    quote: fn(&str) -> String,
) -> String {
    let mut out = String::with_capacity(template.len() + name.len() + path.len());
    let mut rest = template;
    while let Some(idx) = rest.find('{') {
        out.push_str(&rest[..idx]);
        let tail = &rest[idx..];
        if let Some(after) = tail.strip_prefix("{f}") {
            out.push_str(&quote(name));
            rest = after;
        } else if let Some(after) = tail.strip_prefix("{p}") {
            out.push_str(&quote(path));
            rest = after;
        } else {
            out.push('{');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posix_quote_neutralises_injection() {
        assert_eq!(
            quote_posix("/tmp/evil; rm -rf ~ #.txt"),
            "'/tmp/evil; rm -rf ~ #.txt'"
        );
        assert_eq!(quote_posix("it's"), r"'it'\''s'");
    }

    #[test]
    fn msvcrt_rules() {
        assert_eq!(quote_msvcrt("a b"), r#""a b""#);
        assert_eq!(quote_msvcrt(r#"x"y"#), r#""x\"y""#);
        assert_eq!(quote_msvcrt(r"C:\dir\"), r#""C:\dir\\""#);
        assert_eq!(quote_msvcrt(r#"a\"b"#), r#""a\\\"b""#);
        assert_eq!(quote_msvcrt(r"C:\a\b"), r#""C:\a\b""#);
    }

    #[test]
    fn cmd_quote_escapes_every_metachar() {
        assert_eq!(quote_cmd("%PATH%.txt"), r#"^"^%PATH^%.txt^""#);
        assert_eq!(quote_cmd("a&b.txt"), r#"^"a^&b.txt^""#);
        assert_eq!(quote_cmd(r#"x"y"#), r#"^"x\^"y^""#);
        assert_eq!(quote_cmd("100%^!.txt"), r#"^"100^%^^^!.txt^""#);
        assert_eq!(quote_cmd("(a)<b>|c"), r#"^"^(a^)^<b^>^|c^""#);
    }

    #[test]
    fn cmd_quote_never_leaves_an_unescaped_metachar() {
        for name in ["%PATH%.txt", "a&b.txt", r#"x"y"#, "100%^!.txt", "^^&&"] {
            let q = quote_cmd(name);
            let mut chars = q.chars();
            while let Some(c) = chars.next() {
                if c == '^' {
                    assert!(chars.next().is_some(), "{q}");
                } else {
                    assert!(!CMD_METACHARS.contains(&c), "{name} -> {q}");
                }
            }
        }
    }

    #[test]
    fn placeholders_expand_in_single_pass() {
        let out = expand_placeholders("echo {f} {p}", ";id;{p}", "/tmp/;id;{p}", quote_posix);
        assert_eq!(out, "echo ';id;{p}' '/tmp/;id;{p}'");
        let out = expand_placeholders(
            "type {p} & echo {f}",
            "a&calc&{p}",
            r"C:\tmp\a&calc&{p}",
            quote_cmd,
        );
        assert_eq!(
            out,
            r#"type ^"C:\tmp\a^&calc^&{p}^" & echo ^"a^&calc^&{p}^""#
        );
    }

    #[test]
    fn placeholders_keep_unknown_braces() {
        let out = expand_placeholders("{x} {f}{", "n", "p", quote_posix);
        assert_eq!(out, "{x} 'n'{");
    }
}
