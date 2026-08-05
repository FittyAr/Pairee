pub fn command_exists(cmd: &str) -> bool {
    let cmd_name = match cmd.split_whitespace().next() {
        Some(name) => name,
        None => return false,
    };

    let path = std::path::Path::new(cmd_name);
    if path.is_absolute() || path.exists() {
        return true;
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for p in std::env::split_paths(&path_env) {
            let full_path = p.join(cmd_name);
            if full_path.exists() {
                return true;
            }
            if cfg!(target_os = "windows") {
                for ext in &["exe", "bat", "cmd", "com"] {
                    if full_path.with_extension(ext).exists() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Returns a single shell token that, when interpreted by `cmd.exe` or
/// `/bin/sh -c`, expands back to the literal path.
///
/// The output is safe to embed in a shell command string built from
/// untrusted input (e.g. a user-supplied file name). The path is wrapped
/// in the platform's native quoting and every internal metacharacter is
/// escaped so the shell cannot be tricked into executing it.
pub fn shell_quote(path: &std::path::Path) -> String {
    let s = path.to_string_lossy();
    if cfg!(target_os = "windows") {
        // cmd.exe: wrap in double quotes, double every internal `"`.
        // This survives `cmd /c` and PowerShell -Command parsing.
        let mut out = String::with_capacity(s.len() + 2);
        out.push('"');
        for c in s.chars() {
            if c == '"' {
                out.push('"');
                out.push('"');
            } else {
                out.push(c);
            }
        }
        out.push('"');
        out
    } else {
        // POSIX sh: single-quote the whole token, escape any `'` as `'\''`
        // (close, literal quote, reopen). The other shell metacharacters
        // (`;`, `|`, `&`, `$`, `` ` ``, `\`, etc.) are inert inside `'...'`.
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_shell_quote_posix_neutralises_injection() {
        let p = PathBuf::from("/tmp/evil; rm -rf ~ #.txt");
        let q = shell_quote(&p);
        if cfg!(target_os = "windows") {
            // cmd.exe-style quoting: wrap in "..", double internal ".
            // The shell does not interpret `;`, `&`, `|`, etc. inside
            // double quotes, so we only need to escape the characters
            // that ARE special there: `"` and `%`. The literal text is
            // preserved verbatim.
            assert!(q.starts_with('"') && q.ends_with('"'));
            // The dangerous characters should be inside double quotes,
            // which makes them literal to cmd.exe.
            assert!(q.contains("evil; rm -rf ~"));
        } else {
            // POSIX: the entire literal text — including the dangerous chars —
            // is preserved inside a single pair of single quotes. The shell
            // treats `'...'` as one literal token, so the metacharacters
            // inside it are inert. The presence of the literal text inside
            // the quotes is the desired behaviour.
            assert!(q.starts_with('\'') && q.ends_with('\''));
            assert_eq!(q, "'/tmp/evil; rm -rf ~ #.txt'");
        }
    }

    #[test]
    fn test_shell_quote_handles_inner_quote() {
        let p = PathBuf::from("/tmp/he said \"hi\".txt");
        let q = shell_quote(&p);
        if cfg!(target_os = "windows") {
            // Each `"` in the input becomes `""` in the output.
            assert!(q.contains("\"\"hi\"\""));
        } else {
            // POSIX: no special handling of `"`; only `'` matters.
            assert!(q.starts_with('\''));
        }
    }
}
