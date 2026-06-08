use anyhow::{Context, Result};
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
        // Names with spaces must be quoted
        let entry_name = if filename.contains(' ') {
            format!("\"{}\"", filename)
        } else {
            filename.to_string()
        };
        lines.push(format!("{} {}", entry_name, description));
    }

    let output = lines.join("\n") + if lines.is_empty() { "" } else { "\n" };
    std::fs::write(&desc_path, output)
        .with_context(|| format!("Writing {:?}", desc_path))
}

// Expose remove_description utility function for full API completeness.
// Currently validated via unit tests.
/// Removes a file's description entry from the `descript.ion` file.
pub fn remove_description(dir: &Path, filename: &str) -> Result<()> {
    write_description(dir, filename, "")
}

/// Parses a single `descript.ion` line into `(filename, description)`.
/// Handles both quoted and unquoted filenames.
fn parse_description_line(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if line.starts_with('"') {
        // Quoted filename
        let end = line[1..].find('"')? + 1;
        let name = &line[1..end];
        let desc = line[end + 1..].trim_start();
        Some((name, desc))
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
}
