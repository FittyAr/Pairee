//! Text encoding detection: byte-order mark first, then a UTF-16 heuristic,
//! a binary check, strict UTF-8 and finally `chardetng` for legacy code pages.

use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE};

/// Bytes sampled from the start of a file for detection.
pub const DETECT_SAMPLE_BYTES: usize = 64 * 1024;

/// Encodings offered by the viewer's encoding selector (and the default
/// encoding setting), in display order.
pub const ENCODINGS: &[&Encoding] = &[
    UTF_8,
    UTF_16LE,
    UTF_16BE,
    encoding_rs::WINDOWS_1252,
    encoding_rs::ISO_8859_15,
    encoding_rs::WINDOWS_1250,
    encoding_rs::ISO_8859_2,
    encoding_rs::WINDOWS_1251,
    encoding_rs::KOI8_R,
    encoding_rs::IBM866,
    encoding_rs::WINDOWS_1253,
    encoding_rs::WINDOWS_1254,
    encoding_rs::WINDOWS_1255,
    encoding_rs::WINDOWS_1256,
    encoding_rs::WINDOWS_1257,
    encoding_rs::WINDOWS_874,
    encoding_rs::MACINTOSH,
    encoding_rs::SHIFT_JIS,
    encoding_rs::EUC_JP,
    encoding_rs::ISO_2022_JP,
    encoding_rs::GBK,
    encoding_rs::GB18030,
    encoding_rs::BIG5,
    encoding_rs::EUC_KR,
];

/// Share of zero bytes in one byte lane above which a sample looks like
/// UTF-16 (ASCII-range text has a zero high byte in every code unit).
const UTF16_ZERO_LANE_RATIO: f32 = 0.4;
/// Share of zero bytes tolerated in the other lane of UTF-16 text.
const UTF16_OTHER_LANE_RATIO: f32 = 0.05;
/// Fewest code units needed before guessing UTF-16 without a byte-order mark.
const UTF16_MIN_UNITS: usize = 4;

/// How the encoding of a file is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingPolicy {
    /// Detect from the content.
    Detect,
    /// Always use this encoding (a matching byte-order mark is still skipped).
    Fixed(&'static Encoding),
}

/// Outcome of [`detect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Detected {
    pub encoding: &'static Encoding,
    /// Length of the byte-order mark to skip (0 when there is none).
    pub bom_len: usize,
    /// The sample looks like binary data rather than text.
    pub binary: bool,
}

/// Looks an encoding up by its name or any WHATWG label (`"latin1"`,
/// `"cp1252"`, `"shift_jis"`...); bare Windows code page numbers (`"1252"`)
/// are accepted too.
pub fn encoding_by_name(name: &str) -> Option<&'static Encoding> {
    let name = name.trim();
    Encoding::for_label(name.as_bytes())
        .or_else(|| Encoding::for_label(format!("windows-{name}").as_bytes()))
}

/// Length of the byte-order mark at the start of `sample` when it belongs to
/// `encoding`.
pub fn bom_len_for(sample: &[u8], encoding: &'static Encoding) -> usize {
    match Encoding::for_bom(sample) {
        Some((enc, len)) if enc == encoding => len,
        _ => 0,
    }
}

/// Detects the encoding of `sample` (the start of a file; `at_eof` when it is
/// the whole file) according to `policy`.
pub fn detect(sample: &[u8], at_eof: bool, policy: EncodingPolicy) -> Detected {
    if let EncodingPolicy::Fixed(encoding) = policy {
        return Detected {
            encoding,
            bom_len: bom_len_for(sample, encoding),
            binary: !is_utf16(encoding) && sample.contains(&0),
        };
    }
    if let Some((encoding, bom_len)) = Encoding::for_bom(sample) {
        return Detected {
            encoding,
            bom_len,
            binary: false,
        };
    }
    let text = |encoding| Detected {
        encoding,
        bom_len: 0,
        binary: false,
    };
    if let Some(encoding) = guess_utf16(sample) {
        return text(encoding);
    }
    if sample.contains(&0) {
        return Detected {
            binary: true,
            ..text(UTF_8)
        };
    }
    match std::str::from_utf8(sample) {
        Ok(_) => text(UTF_8),
        // Only a character cut by the end of the sample.
        Err(e) if e.error_len().is_none() && !at_eof => text(UTF_8),
        Err(_) => {
            let mut detector = chardetng::EncodingDetector::new();
            detector.feed(sample, at_eof);
            text(detector.guess(None, true))
        }
    }
}

pub fn is_utf16(encoding: &'static Encoding) -> bool {
    encoding == UTF_16LE || encoding == UTF_16BE
}

/// UTF-16 without a byte-order mark: one byte lane is mostly zero (the high
/// byte of Latin text) while the other almost never is.
fn guess_utf16(sample: &[u8]) -> Option<&'static Encoding> {
    let units = sample.len() / 2;
    if units < UTF16_MIN_UNITS {
        return None;
    }
    let zeros = |lane: usize| {
        sample
            .iter()
            .skip(lane)
            .step_by(2)
            .take(units)
            .filter(|&&b| b == 0)
            .count() as f32
            / units as f32
    };
    let (even, odd) = (zeros(0), zeros(1));
    let guess = if odd >= UTF16_ZERO_LANE_RATIO && even <= UTF16_OTHER_LANE_RATIO {
        UTF_16LE
    } else if even >= UTF16_ZERO_LANE_RATIO && odd <= UTF16_OTHER_LANE_RATIO {
        UTF_16BE
    } else {
        return None;
    };
    // Binary data with a zero lane decodes to control characters.
    let decoded = guess.decode_without_bom_handling(&sample[..units * 2]).0;
    decoded.chars().all(is_text_char).then_some(guess)
}

/// Printable characters and the usual whitespace controls.
fn is_text_char(c: char) -> bool {
    !c.is_control() || matches!(c, '\t' | '\n' | '\r' | '\x0C')
}

/// Decodes the start of a file (`at_eof` when `bytes` is the whole file) for
/// a preview. `None` means binary. A character cut by the end of `bytes` is
/// dropped instead of becoming a replacement character.
pub fn decode_prefix(bytes: &[u8], at_eof: bool, policy: EncodingPolicy) -> Option<String> {
    let detected = detect(bytes, at_eof, policy);
    if detected.binary {
        return None;
    }
    let body = &bytes[detected.bom_len..];
    let mut decoder = detected.encoding.new_decoder_without_bom_handling();
    let capacity = decoder.max_utf8_buffer_length(body.len())?;
    let mut out = String::with_capacity(capacity);
    let _ = decoder.decode_to_string(body, &mut out, at_eof);
    Some(out)
}

/// Decodes one line (no byte-order mark, no terminator).
pub fn decode_line(encoding: &'static Encoding, bytes: &[u8]) -> String {
    encoding.decode_without_bom_handling(bytes).0.into_owned()
}
