//! Line splitting over a [`ByteStore`], chunk by chunk.
//!
//! One splitter serves the background line index, the viewer's visible lines
//! and the search, so they always agree on where lines start. Lines end at LF
//! (a preceding CR is dropped, even when CR and LF sit in different chunks);
//! lines longer than [`MAX_LINE_BYTES`] are split at a character boundary so
//! a file without line breaks still pages in bounded steps.

use super::encoding::is_utf16;
use super::store::ByteStore;
use encoding_rs::{Encoding, UTF_8, UTF_16BE};
use std::io;
use std::ops::ControlFlow;

/// Longest line handed out in one piece; longer lines continue on the next.
pub const MAX_LINE_BYTES: usize = 8 * 1024;

const LF: u16 = 0x0A;
const CR: u16 = 0x0D;

/// Code-unit layout of an encoding, as far as line splitting cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineFormat {
    /// ASCII-compatible encoding: LF is the byte 0x0A and never part of a
    /// multi-byte character (true for UTF-8, Windows/ISO code pages,
    /// Shift-JIS, EUC, GBK, Big5).
    Bytes { utf8: bool },
    /// 16-bit code units.
    Utf16 { big_endian: bool },
}

impl LineFormat {
    pub fn for_encoding(encoding: &'static Encoding) -> Self {
        if is_utf16(encoding) {
            Self::Utf16 {
                big_endian: encoding == UTF_16BE,
            }
        } else {
            Self::Bytes {
                utf8: encoding == UTF_8,
            }
        }
    }

    /// Bytes per code unit.
    pub fn width(self) -> usize {
        match self {
            Self::Bytes { .. } => 1,
            Self::Utf16 { .. } => 2,
        }
    }

    /// Code unit starting at `b[0]` (`b` holds at least [`Self::width`] bytes).
    fn unit(self, b: &[u8]) -> u16 {
        match self {
            Self::Bytes { .. } => u16::from(b[0]),
            Self::Utf16 { big_endian: false } => u16::from_le_bytes([b[0], b[1]]),
            Self::Utf16 { big_endian: true } => u16::from_be_bytes([b[0], b[1]]),
        }
    }

    /// Whether a long line may be cut right before the unit `b`.
    fn can_split_before(self, b: &[u8]) -> bool {
        match self {
            Self::Bytes { utf8: true } => b[0] & 0xC0 != 0x80,
            Self::Bytes { utf8: false } => true,
            // Never between the halves of a surrogate pair.
            Self::Utf16 { .. } => !(0xDC00..=0xDFFF).contains(&self.unit(b)),
        }
    }

    /// `content` without a trailing CR.
    fn strip_cr(self, content: &[u8]) -> &[u8] {
        let w = self.width();
        match content.len().checked_sub(w) {
            Some(cut) if self.unit(&content[cut..]) == CR => &content[..cut],
            _ => content,
        }
    }
}

/// Calls `f(line_start, content)` for every line from `from` (which must be a
/// line start) to the end of `store`, reading `chunk` bytes at a time
/// (`chunk` must be a multiple of the unit width). `content` excludes the
/// terminator. Stops early when `f` breaks.
pub fn for_each_line<F>(
    store: &dyn ByteStore,
    format: LineFormat,
    from: u64,
    chunk: usize,
    mut f: F,
) -> io::Result<()>
where
    F: FnMut(u64, &[u8]) -> ControlFlow<()>,
{
    let w = format.width();
    let mut buf = vec![0u8; chunk.max(w)];
    let mut pending: Vec<u8> = Vec::new();
    let mut line_start = from;
    let mut pos = from;
    while pos < store.len() {
        let n = store.read_at(pos, &mut buf)?;
        if n == 0 {
            break;
        }
        let data = &buf[..n];
        let mut seg = 0;
        let mut i = 0;
        while i + w <= n {
            let unit = &data[i..i + w];
            let cut = if format.unit(unit) == LF {
                Some((i, i + w, true))
            } else if pending.len() + (i - seg) >= MAX_LINE_BYTES && format.can_split_before(unit) {
                Some((i, i, false))
            } else {
                None
            };
            if let Some((end, next, is_lf)) = cut {
                let content = joined(&mut pending, &data[seg..end]);
                let content = if is_lf {
                    format.strip_cr(content)
                } else {
                    content
                };
                if f(line_start, content).is_break() {
                    return Ok(());
                }
                pending.clear();
                seg = next;
                line_start = pos + next as u64;
            }
            i += w;
        }
        pending.extend_from_slice(&data[seg..]);
        pos += n as u64;
    }
    if !pending.is_empty() {
        let _ = f(line_start, &pending);
    }
    Ok(())
}

/// `pending + tail` without copying when nothing is pending.
fn joined<'a>(pending: &'a mut Vec<u8>, tail: &'a [u8]) -> &'a [u8] {
    if pending.is_empty() {
        tail
    } else {
        pending.extend_from_slice(tail);
        pending
    }
}
