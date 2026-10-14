//! On-disk representation of a text file opened in the built-in editor.
//!
//! The editor works on a `Vec<String>` of lines. This module converts between
//! that and the bytes on disk while remembering the details needed to write
//! the file back unchanged: line ending style, trailing newline and UTF-8 BOM.

use std::io;
use std::path::Path;
use std::time::SystemTime;

/// Largest file the built-in editor opens (same cap as the viewer).
pub const EDITOR_MAX_BYTES: u64 = crate::ui::viewer::VIEWER_MAX_BYTES;

const UTF8_BOM: &str = "\u{feff}";

/// Line terminator used when the file is written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    #[default]
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }

    /// Short label for the status line.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::CrLf => "CRLF",
        }
    }
}

/// Formatting details preserved across load → save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextFormat {
    pub line_ending: LineEnding,
    pub trailing_newline: bool,
    pub bom: bool,
}

/// File-system facts captured when the file was read or last written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiskStamp {
    pub modified: Option<SystemTime>,
    pub read_only: bool,
}

impl DiskStamp {
    /// Current stamp of `path`; a missing file yields the default stamp.
    pub fn of(path: &Path) -> Self {
        std::fs::metadata(path)
            .map(|m| Self {
                modified: m.modified().ok(),
                read_only: m.permissions().readonly(),
            })
            .unwrap_or_default()
    }
}

/// Why a file could not be opened in the editor.
#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    TooLarge(u64),
    NotUtf8,
}

/// A file read from disk, split into editor lines.
#[derive(Debug, Clone)]
pub struct LoadedText {
    pub lines: Vec<String>,
    pub format: TextFormat,
    pub stamp: DiskStamp,
}

/// Splits `text` into lines and detects its format. A file is CRLF when
/// every line ends in `\r\n` (the `\r` then never stays in a line); with
/// mixed endings it is LF and each `\r` stays in its line, so saving
/// writes back exactly what was read.
pub fn parse(text: &str) -> (Vec<String>, TextFormat) {
    let (bom, body) = match text.strip_prefix(UTF8_BOM) {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let newlines = body.matches('\n').count();
    let line_ending = if newlines > 0 && body.matches("\r\n").count() == newlines {
        LineEnding::CrLf
    } else {
        LineEnding::Lf
    };
    let trailing_newline = body.ends_with('\n');
    let content = body.strip_suffix('\n').unwrap_or(body);
    let lines = content
        .split('\n')
        .map(|l| match line_ending {
            LineEnding::CrLf => l.strip_suffix('\r').unwrap_or(l).to_string(),
            LineEnding::Lf => l.to_string(),
        })
        .collect();
    (
        lines,
        TextFormat {
            line_ending,
            trailing_newline,
            bom,
        },
    )
}

/// Joins `lines` back into file content using `format`.
pub fn serialize(lines: &[String], format: &TextFormat) -> String {
    let mut out = String::new();
    if format.bom {
        out.push_str(UTF8_BOM);
    }
    out.push_str(&lines.join(format.line_ending.as_str()));
    if format.trailing_newline {
        out.push_str(format.line_ending.as_str());
    }
    out
}

/// Reads `path` for editing. Files that are not valid UTF-8 are refused:
/// saving a lossy decoding would corrupt them.
pub fn load(path: &Path) -> Result<LoadedText, LoadError> {
    let size = std::fs::metadata(path).map_err(LoadError::Io)?.len();
    if size > EDITOR_MAX_BYTES {
        return Err(LoadError::TooLarge(size));
    }
    let bytes = std::fs::read(path).map_err(LoadError::Io)?;
    let text = String::from_utf8(bytes).map_err(|_| LoadError::NotUtf8)?;
    let (lines, format) = parse(&text);
    Ok(LoadedText {
        lines,
        format,
        stamp: DiskStamp::of(path),
    })
}

/// Writes `lines` to `path` and returns the new disk stamp.
///
/// The file is replaced atomically, keeping its permissions; a symbolic
/// link is followed (the link stays and its target is written), and a file
/// with other hard links is rewritten in place so every name sees the text.
pub fn save(path: &Path, lines: &[String], format: &TextFormat) -> anyhow::Result<DiskStamp> {
    let data = serialize(lines, format);
    let target = link_target(path);
    let existing = std::fs::metadata(&target).ok();
    if existing.as_ref().is_some_and(has_other_links) {
        std::fs::write(&target, data)?;
    } else {
        crate::config::write_atomic(&target, data.as_bytes())?;
        if let Some(meta) = existing {
            std::fs::set_permissions(&target, meta.permissions())?;
        }
    }
    Ok(DiskStamp::of(path))
}

/// The file a symbolic link at `path` points to (`path` itself otherwise,
/// or when the link is broken).
fn link_target(path: &Path) -> std::path::PathBuf {
    let is_link = std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink());
    match is_link {
        true => std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
        false => path.to_path_buf(),
    }
}

#[cfg(unix)]
fn has_other_links(meta: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    meta.nlink() > 1
}

#[cfg(not(unix))]
fn has_other_links(_meta: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(text: &str) {
        let (lines, format) = parse(text);
        assert_eq!(serialize(&lines, &format), text, "roundtrip of {text:?}");
    }

    #[test]
    fn roundtrips_preserve_bytes() {
        for text in [
            "",
            "a",
            "a\n",
            "a\nb",
            "a\r\nb\r\n",
            "\u{feff}héllo\r\nwörld",
            "\n\n",
            "tab\there\n",
            "mixed\nline\r\nends",
            "a\r\nb\nc\r\n",
        ] {
            roundtrip(text);
        }
    }

    #[test]
    fn detects_crlf_and_strips_cr() {
        let (lines, format) = parse("one\r\ntwo\r\n");
        assert_eq!(lines, vec!["one", "two"]);
        assert_eq!(format.line_ending, LineEnding::CrLf);
        assert!(format.trailing_newline);
        assert!(!format.bom);
    }

    #[test]
    fn empty_file_has_one_empty_line() {
        let (lines, format) = parse("");
        assert_eq!(lines, vec![String::new()]);
        assert!(!format.trailing_newline);
    }

    #[test]
    fn load_rejects_invalid_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin.dat");
        std::fs::write(&path, [0xff, 0xfe, 0x00, 0x80]).unwrap();
        assert!(matches!(load(&path), Err(LoadError::NotUtf8)));
    }

    #[test]
    fn save_then_load_keeps_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("win.txt");
        std::fs::write(&path, "a\r\nb\r\n").unwrap();
        let loaded = load(&path).unwrap();
        let mut lines = loaded.lines;
        lines[1].push('!');
        save(&path, &lines, &loaded.format).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a\r\nb!\r\n");
    }

    #[cfg(unix)]
    #[test]
    fn save_keeps_mode_symlinks_and_hard_links() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("run.sh");
        std::fs::write(&script, "echo a\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let link = dir.path().join("link.sh");
        std::os::unix::fs::symlink(&script, &link).unwrap();
        let hard = dir.path().join("hard.sh");
        std::fs::hard_link(&script, &hard).unwrap();
        let (lines, format) = parse("echo b\n");

        save(&link, &lines, &format).unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read_to_string(&script).unwrap(), "echo b\n");
        assert_eq!(std::fs::read_to_string(&hard).unwrap(), "echo b\n");
        let mode = std::fs::metadata(&script).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);

        std::fs::remove_file(&hard).unwrap();
        save(&script, &parse("echo c\n").0, &format).unwrap();
        let mode = std::fs::metadata(&script).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755, "atomic replace keeps the mode");
    }
}
