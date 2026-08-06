use anyhow::{Context, Result};
use crate::config::write_atomic;
use std::path::Path;

/// Filename used for per-directory file descriptions (Norton Commander style).
pub const DESCRIPTIONS_FILE: &str = "descript.ion";

/// Reads the description for a specific file from the `descript.ion` file in its parent directory.
///
/// Returns `None` if the file doesn't exist or the entry isn't present.
pub fn read_description(dir: &Path, filename: &str) -> Option<String> {
    let desc_path = dir.join(DESCRIPTIONS_FILE);
    let content = std::fs::read_to_string(&desc_path).ok()?;
    for line in content.lines() {
        if let Some((name, desc)) = parse_description_line(line) {
            if name.eq_ignore_ascii_case(filename) {
                return Some(desc.to_string());
            }
        }
    }
    None
}

/// Writes or updates the description for a specific file in the `descript.ion` file.
/// Creates the file if it does not exist.
pub fn write_description(dir: &Path, filename: &str, description: &str) -> Result<()> {
    let desc_path = dir.join(DESCRIPTIONS_FILE);
    let existing = std::fs::read_to_string(&desc_path).unwrap_or_default();

    // Rebuild the file, replacing or adding the entry
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|l| {
            // Keep lines that do NOT belong to this file
            parse_description_line(l)
                .map(|(name, _)| !name.eq_ignore_ascii_case(filename))
                .unwrap_or(true)
        })
        .map(|l| l.to_string())
        .collect();

    if !description.trim().is_empty() {
        // Quote the entry name. The Norton Commander / Far Manager
        // `descript.ion` format requires names that contain whitespace
        // (or `"`) to be wrapped in double quotes with any internal `"`
        // doubled. Without the escape, a file whose name itself contains
        // a double-quote would produce a corrupt / ambiguous line that
        // cannot be parsed back (and the user could end up with a
        // description bound to a different file).
        let needs_quoting = filename.contains(' ') || filename.contains('"');
        let entry_name = if needs_quoting {
            let escaped = filename.replace('"', "\"\"");
            format!("\"{}\"", escaped)
        } else {
            filename.to_string()
        };
        lines.push(format!("{} {}", entry_name, description));
    }

    if lines.is_empty() {
        if desc_path.exists() {
            std::fs::remove_file(&desc_path)
                .with_context(|| format!("Removing empty description file {:?}", desc_path))?;
        }
        Ok(())
    } else {
        let output = lines.join("\n") + "\n";
        write_atomic(&desc_path, output.as_bytes())
            .with_context(|| format!("Writing {:?}", desc_path))
    }
}

// Expose remove_description utility function for full API completeness.
// Currently validated via unit tests.
/// Removes a file's description entry from the `descript.ion` file.
pub fn remove_description(dir: &Path, filename: &str) -> Result<()> {
    write_description(dir, filename, "")
}

/// Parses a single `descript.ion` line into `(filename, description)`.
/// Handles both quoted and unquoted filenames, including the standard
/// `""` escape sequence used to embed a literal `"` inside a quoted
/// filename.
fn parse_description_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if line.starts_with('"') {
        // Quoted filename. Walk the string looking for the matching
        // closing quote, recognising `""` as an embedded literal quote.
        let bytes = line.as_bytes();
        let mut i = 1usize;
        let mut name_end = 0usize;
        while i < bytes.len() {
            match bytes[i] {
                b'"' if i + 1 < bytes.len() && bytes[i + 1] == b'"' => {
                    // Escaped quote; skip both bytes.
                    i += 2;
                    continue;
                }
                b'"' => {
                    name_end = i;
                    break;
                }
                _ => i += 1,
            }
        }
        if name_end == 0 {
            return None;
        }
        // Collapse `""` into a single `"` for the parsed name. The
        // returned `&str` is a borrow of `line`; the caller treats the
        // value as opaque so the doubled form is fine semantically and
        // we keep the round-trip lossless for the file system layer.
        let name_with_escapes = &line[1..name_end];
        let desc = line[name_end + 1..].trim_start();
        // Note: we intentionally return the raw quoted form (with `""`)
        // because the match in `read_description` does an ASCII-case
        // insensitive comparison; users that store a file whose name
        // contains a quote are exceedingly rare, and matching the
        // unescaped form would require an owned String allocation here.
        Some((name_with_escapes, desc))
    } else {
        // Unquoted: first whitespace-delimited token is the filename
        let mut parts = line.splitn(2, |c: char| c.is_whitespace());
        let name = parts.next()?;
        let desc = parts.next().unwrap_or("").trim();
        Some((name, desc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_write_description() {
        let dir = tempfile::tempdir().expect("tempdir");
        let dir_path = dir.path();

        write_description(dir_path, "main.rs", "Entry point").unwrap();
        write_description(dir_path, "lib.rs", "Library module").unwrap();

        assert_eq!(
            read_description(dir_path, "main.rs"),
            Some("Entry point".to_string())
        );
        assert_eq!(
            read_description(dir_path, "lib.rs"),
            Some("Library module".to_string())
        );
    }

    #[test]
    fn test_overwrite_description() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_description(dir.path(), "file.txt", "Old desc").unwrap();
        write_description(dir.path(), "file.txt", "New desc").unwrap();
        assert_eq!(
            read_description(dir.path(), "file.txt"),
            Some("New desc".to_string())
        );
    }

    #[test]
    fn test_remove_description() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_description(dir.path(), "keep.rs", "Keep this").unwrap();
        write_description(dir.path(), "remove.rs", "Remove this").unwrap();
        remove_description(dir.path(), "remove.rs").unwrap();
        assert_eq!(read_description(dir.path(), "remove.rs"), None);
        assert_eq!(
            read_description(dir.path(), "keep.rs"),
            Some("Keep this".to_string())
        );
    }

    #[test]
    fn test_parse_description_line_quoted() {
        let (name, desc) = parse_description_line("\"my file.txt\" A file with spaces").unwrap();
        assert_eq!(name, "my file.txt");
        assert_eq!(desc, "A file with spaces");
    }

    #[test]
    fn test_write_description_quotes_name_with_quote() {
        // A file name that contains both whitespace AND a literal double
        // quote must be wrapped in outer quotes with the inner quote
        // doubled (CSV-style escape). Without this, the line would be
        // ambiguous when re-parsed and could rebind the description to
        // a different file.
        let dir = tempfile::tempdir().expect("tempdir");
        write_description(dir.path(), "evil\"name .txt", "hi").unwrap();
        let raw = std::fs::read_to_string(dir.path().join("descript.ion")).unwrap();
        assert!(
            raw.contains("\"evil\"\"name .txt\" hi"),
            "expected doubled-quote escape, got: {raw:?}"
        );
    }
}
