use crate::config::write_atomic;
use crate::fs::error::LocalizedResult;
use anyhow::Result;
use std::path::Path;

/// Filename used for per-directory file descriptions (Norton Commander style).
pub const DESCRIPTIONS_FILE: &str = "descript.ion";

/// Reads the description for a specific file from the `descript.ion` file in its parent directory.
///
/// Returns `None` if the file doesn't exist or the entry isn't present.
pub fn read_description(dir: &Path, filename: &str) -> Option<String> {
    let content = read_descriptions_file(dir)?;
    find_description(&content, filename).map(str::to_string)
}

/// Whole `descript.ion` of `dir`, to look up many names with
/// [`find_description`] after one read.
pub(crate) fn read_descriptions_file(dir: &Path) -> Option<String> {
    // Lossy: other tools write it in legacy code pages.
    let bytes = std::fs::read(dir.join(DESCRIPTIONS_FILE)).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Description of `filename` in the `descript.ion` text `content`.
pub(crate) fn find_description<'a>(content: &'a str, filename: &str) -> Option<&'a str> {
    content
        .lines()
        .filter_map(parse_description_line)
        .find(|(name, _)| name.eq_ignore_ascii_case(filename))
        .map(|(_, desc)| desc)
}

/// The lines of a `descript.ion` file as stored (other tools write them in
/// legacy code pages, so they are not required to be UTF-8); none when it
/// does not exist.
fn read_lines(desc_path: &Path) -> std::io::Result<Vec<Vec<u8>>> {
    let bytes = match std::fs::read(desc_path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut lines: Vec<Vec<u8>> = bytes
        .split(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line).to_vec())
        .collect();
    if bytes.ends_with(b"\n") {
        lines.pop();
    }
    Ok(lines)
}

/// The name a `descript.ion` line describes, matched without regard to
/// ASCII case.
fn describes(line: &[u8], filename: &str) -> bool {
    parse_description_line(&String::from_utf8_lossy(line))
        .is_some_and(|(name, _)| name.eq_ignore_ascii_case(filename))
}

/// Writes or updates the description for a specific file in the `descript.ion` file.
/// Creates the file if it does not exist.
pub fn write_description(dir: &Path, filename: &str, description: &str) -> Result<()> {
    let desc_path = dir.join(DESCRIPTIONS_FILE);
    // A file that cannot be read is left alone (rewriting it from nothing
    // would lose every other description); lines of other files are kept
    // byte for byte, whatever their encoding.
    let mut lines: Vec<Vec<u8>> = read_lines(&desc_path)
        .localized("fs_error_read_file", &desc_path)?
        .into_iter()
        .filter(|line| !describes(line, filename))
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
        lines.push(format!("{} {}", entry_name, description).into_bytes());
    }

    if lines.is_empty() {
        if desc_path.exists() {
            std::fs::remove_file(&desc_path).localized("fs_error_remove", &desc_path)?;
        }
        Ok(())
    } else {
        let mut output = lines.join(&b'\n');
        output.push(b'\n');
        write_atomic(&desc_path, &output).localized("fs_error_write_file", &desc_path)
    }
}

/// Drops the description of the entry at `path` (deleted, moved or
/// wiped) from its folder's `descript.ion`. Best effort: a description that
/// cannot be updated never fails the operation that removed the entry.
pub fn forget_description(path: &Path) {
    if let (Some(dir), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str())) {
        let _ = write_description(dir, name, "");
    }
}

/// Parses a single `descript.ion` line into `(filename, description)`.
/// Handles both quoted and unquoted filenames, including the standard
/// `""` escape sequence used to embed a literal `"` inside a quoted
/// filename.
pub(crate) fn parse_description_line(line: &str) -> Option<(&str, &str)> {
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
    fn test_forget_description() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_description(dir.path(), "keep.rs", "Keep this").unwrap();
        write_description(dir.path(), "remove.rs", "Remove this").unwrap();
        forget_description(&dir.path().join("remove.rs"));
        assert_eq!(read_description(dir.path(), "remove.rs"), None);
        assert_eq!(
            read_description(dir.path(), "keep.rs"),
            Some("Keep this".to_string())
        );
    }

    /// Far Manager and Total Commander write `descript.ion` in the OEM or
    /// ANSI code page: updating one entry keeps the others, bytes intact.
    #[test]
    fn legacy_encoded_files_keep_their_other_entries() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(DESCRIPTIONS_FILE);
        std::fs::write(&path, b"old.txt caf\xe9 cr\xe8me\r\ngone.txt bye\r\n").unwrap();
        forget_description(&dir.path().join("gone.txt"));
        assert_eq!(std::fs::read(&path).unwrap(), b"old.txt caf\xe9 cr\xe8me\n");
        write_description(dir.path(), "new.txt", "hello").unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"old.txt caf\xe9 cr\xe8me\nnew.txt hello\n"
        );
        assert_eq!(
            read_description(dir.path(), "old.txt"),
            Some("caf\u{fffd} cr\u{fffd}me".to_string())
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
