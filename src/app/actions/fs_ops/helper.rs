pub fn command_exists(cmd: &str) -> bool {
    let Some(cmd_name) = split_command_line(cmd).into_iter().next() else {
        return false;
    };

    let path = std::path::Path::new(&cmd_name);
    if path.is_absolute() || path.exists() {
        return true;
    }

    if let Ok(path_env) = std::env::var("PATH") {
        for p in std::env::split_paths(&path_env) {
            let full_path = p.join(&cmd_name);
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

/// Split a user command line into argv tokens.
///
/// Unix uses POSIX `shlex` (quoted words, backslash escapes). Windows does
/// not treat `\` as an escape (that would mangle `C:\Program Files\...`);
/// it only honours double-quoted spans.
pub fn split_command_line(s: &str) -> Vec<String> {
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }
    #[cfg(unix)]
    {
        shlex::split(s).unwrap_or_else(|| s.split_whitespace().map(str::to_string).collect())
    }
    #[cfg(not(unix))]
    {
        split_windows_quoted(s)
    }
}

#[cfg(not(unix))]
fn split_windows_quoted(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for c in s.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            c if c.is_whitespace() && !in_quotes => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_command_line_keeps_quoted_program() {
        let parts = split_command_line(r#""C:\Program Files\App\app.exe" --flag"#);
        assert_eq!(
            parts,
            vec![
                r"C:\Program Files\App\app.exe".to_string(),
                "--flag".to_string()
            ]
        );
    }

    #[test]
    fn split_command_line_empty_and_simple() {
        assert!(split_command_line("").is_empty());
        assert_eq!(
            split_command_line("nano --lint"),
            vec!["nano".to_string(), "--lint".to_string()]
        );
    }
}
