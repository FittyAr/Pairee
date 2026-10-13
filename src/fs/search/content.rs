//! "Containing text": whether a file holds the searched text, whatever its
//! encoding. Text files are decoded with the encoding the viewer would
//! detect (UTF-8, UTF-16, legacy code pages) and searched with the
//! viewer's matcher; binary files are searched byte by byte. Invalid bytes
//! never end the search early.

use crate::fs::text::encoding::{EncodingPolicy, detect};
use crate::fs::text::search::Matcher;
use std::io::Read;
use std::path::Path;

/// Larger files are not searched.
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Bytes read per step (cancellation is checked between steps).
const CHUNK: usize = 1024 * 1024;

/// `true` when the file at `path` contains `needle`. `cancelled` is polled
/// while reading.
pub fn file_contains(
    path: &Path,
    needle: &str,
    case_sensitive: bool,
    cancelled: &dyn Fn() -> bool,
) -> bool {
    let Some(bytes) = read_small(path, cancelled) else {
        return false;
    };
    let detected = detect(&bytes, true, EncodingPolicy::Detect);
    if detected.binary {
        return bytes_contain(&bytes, needle.as_bytes(), case_sensitive);
    }
    let (text, _, _) = detected.encoding.decode(&bytes);
    Matcher::new(needle, case_sensitive).matches(&text)
}

/// The whole file, when it is small enough and the search goes on.
fn read_small(path: &Path, cancelled: &dyn Fn() -> bool) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_FILE_BYTES {
        return None;
    }
    let mut reader = file.take(MAX_FILE_BYTES);
    let mut bytes = Vec::new();
    let mut chunk = vec![0u8; CHUNK];
    loop {
        if cancelled() {
            return None;
        }
        let n = reader.read(&mut chunk).ok()?;
        if n == 0 {
            return Some(bytes);
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
}

/// Byte search (ASCII case folding when not case-sensitive).
fn bytes_contain(haystack: &[u8], needle: &[u8], case_sensitive: bool) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack.windows(needle.len()).any(|window| {
        if case_sensitive {
            window == needle
        } else {
            window.eq_ignore_ascii_case(needle)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains(bytes: &[u8], needle: &str) -> bool {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, bytes).unwrap();
        file_contains(&path, needle, false, &|| false)
    }

    /// Regression (FS-11): invalid UTF-8 stopped the search at that line.
    #[test]
    fn finds_text_after_invalid_utf8_and_in_legacy_encodings() {
        let mut latin1 = b"first line\n\xe9t\xe9 caf\xe9\n".to_vec();
        latin1.extend_from_slice(b"needle here\n");
        assert!(contains(&latin1, "needle"));
        assert!(contains(&latin1, "CAFÉ"));
        let utf16: Vec<u8> = "\u{feff}hello wörld"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(contains(&utf16, "WÖRLD"));
        assert!(!contains(b"plain text", "missing"));
    }

    #[test]
    fn binary_files_are_searched_by_bytes() {
        assert!(contains(b"\x00\x01binary\x00Marker\xff", "marker"));
        assert!(!contains(b"\x00\x01binary\x00", "marker"));
    }

    #[test]
    fn cancellation_stops_reading() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"needle").unwrap();
        assert!(!file_contains(&path, "needle", true, &|| true));
    }
}
