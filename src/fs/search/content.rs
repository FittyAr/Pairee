//! "Containing text": whether a file holds the searched text, whatever its
//! encoding. Text files are decoded with the encoding the viewer would
//! detect (UTF-8, UTF-16, legacy code pages) and searched with the
//! viewer's matcher; binary files are searched byte by byte. Invalid bytes
//! never end the search early. Files of any size are searched as a stream,
//! one chunk at a time, with the end of each chunk carried into the next so
//! a match across the boundary is still found.

use crate::fs::text::encoding::{EncodingPolicy, detect};
use crate::fs::text::search::Matcher;
use std::io::Read;
use std::path::Path;

/// Bytes read per step (cancellation is checked between steps; the
/// encoding is detected on the first one).
const CHUNK: usize = 1024 * 1024;

/// `true` when the file at `path` contains `needle`. `cancelled` is polled
/// while reading.
pub fn file_contains(
    path: &Path,
    needle: &str,
    case_sensitive: bool,
    cancelled: &dyn Fn() -> bool,
) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut chunk = vec![0u8; CHUNK];
    let Some(first) = read_chunk(&mut file, &mut chunk, cancelled) else {
        return false;
    };
    let detected = detect(&chunk[..first], first < CHUNK, EncodingPolicy::Detect);
    let mut search: Box<dyn Search> = if detected.binary {
        Box::new(BytesSearch::new(needle.as_bytes(), case_sensitive))
    } else {
        Box::new(TextSearch {
            decoder: detected.encoding.new_decoder_without_bom_handling(),
            matcher: Matcher::new(needle, case_sensitive),
            // Case folding can lengthen text: keep a margin.
            keep_chars: needle.chars().count() * 2,
            carry: String::new(),
        })
    };
    let mut data = &chunk[detected.bom_len.min(first)..first];
    let mut last = first < CHUNK;
    loop {
        if search.feed(data, last) {
            return true;
        }
        if last {
            return false;
        }
        let Some(n) = read_chunk(&mut file, &mut chunk, cancelled) else {
            return false;
        };
        data = &chunk[..n];
        last = n < CHUNK;
    }
}

/// Fills `buf` as far as the file allows; `None` when reading failed or
/// the search was cancelled.
fn read_chunk(
    file: &mut std::fs::File,
    buf: &mut [u8],
    cancelled: &dyn Fn() -> bool,
) -> Option<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        if cancelled() {
            return None;
        }
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
    Some(filled)
}

/// A search fed one chunk at a time.
trait Search {
    /// `true` once the needle was seen (`last`: no more data follows).
    fn feed(&mut self, data: &[u8], last: bool) -> bool;
}

/// Text decoded as a stream (a character cut by a chunk boundary is
/// completed by the next chunk).
struct TextSearch {
    decoder: encoding_rs::Decoder,
    matcher: Matcher,
    keep_chars: usize,
    /// The end of the text seen so far.
    carry: String,
}

impl Search for TextSearch {
    fn feed(&mut self, data: &[u8], last: bool) -> bool {
        let capacity = self
            .decoder
            .max_utf8_buffer_length(data.len())
            .unwrap_or(data.len() * 3);
        let mut text = std::mem::take(&mut self.carry);
        text.reserve(capacity);
        let _ = self.decoder.decode_to_string(data, &mut text, last);
        if self.matcher.matches(&text) {
            return true;
        }
        let cut = text
            .char_indices()
            .rev()
            .nth(self.keep_chars.saturating_sub(1))
            .map_or(0, |(i, _)| i);
        self.carry = text.split_off(cut);
        false
    }
}

/// Raw bytes (ASCII case folding when not case-sensitive).
struct BytesSearch {
    needle: Vec<u8>,
    case_sensitive: bool,
    carry: Vec<u8>,
}

impl BytesSearch {
    fn new(needle: &[u8], case_sensitive: bool) -> Self {
        Self {
            needle: needle.to_vec(),
            case_sensitive,
            carry: Vec::new(),
        }
    }
}

impl Search for BytesSearch {
    fn feed(&mut self, data: &[u8], _last: bool) -> bool {
        if self.needle.is_empty() {
            return true;
        }
        let mut haystack = std::mem::take(&mut self.carry);
        haystack.extend_from_slice(data);
        let found = haystack.windows(self.needle.len()).any(|window| {
            if self.case_sensitive {
                window == self.needle.as_slice()
            } else {
                window.eq_ignore_ascii_case(&self.needle)
            }
        });
        let keep = self.needle.len() - 1;
        self.carry = haystack[haystack.len().saturating_sub(keep)..].to_vec();
        found
    }
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

    /// Big files are searched too (they used to be skipped past 16 MiB),
    /// and a match cut by a chunk boundary is found.
    #[test]
    fn large_files_and_matches_across_chunks_are_found() {
        let mut text = "x".repeat(17 * 1024 * 1024).into_bytes();
        text.extend_from_slice("end ñandú".as_bytes());
        assert!(contains(&text, "ÑANDÚ"));

        let mut across = vec![b'a'; CHUNK - 3];
        across.extend_from_slice("café needle".as_bytes());
        assert!(contains(&across, "café needle"));

        let mut binary = vec![0u8; CHUNK - 2];
        binary.extend_from_slice(b"Marker");
        assert!(contains(&binary, "marker"));
        assert!(!contains(&binary, "other"));
    }
}
