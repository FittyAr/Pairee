//! Minimal text extraction from PDF content streams (Flate-compressed).

use flate2::read::ZlibDecoder;
use std::io::Read;

/// Total bytes inflated across all streams. Flate reaches ratios above
/// 1000:1, so without a cap a small crafted PDF exhausts memory.
const MAX_INFLATED_BYTES: u64 = 16 * 1024 * 1024;

pub fn extract_pdf_text(data: &[u8]) -> Option<String> {
    let mut text_content = String::new();
    let mut pos = 0;
    let mut inflate_budget = MAX_INFLATED_BYTES;
    while inflate_budget > 0
        && let Some(stream_start) = find_subsequence(&data[pos..], b"stream")
    {
        let actual_start = pos + stream_start + 6;
        let mut data_start = actual_start;
        while data_start < data.len() && (data[data_start] == b'\r' || data[data_start] == b'\n') {
            data_start += 1;
        }

        if let Some(stream_end) = find_subsequence(&data[data_start..], b"endstream") {
            let actual_end = data_start + stream_end;
            let compressed_data = &data[data_start..actual_end];

            let mut decoder = ZlibDecoder::new(compressed_data).take(inflate_budget);
            let mut decompressed = Vec::new();
            let inflated = decoder.read_to_end(&mut decompressed);
            inflate_budget -= decompressed.len() as u64;
            if inflated.is_ok() {
                let mut i = 0;
                let mut in_string = false;
                let mut current_str = Vec::new();
                let mut escaped = false;
                while i < decompressed.len() {
                    let c = decompressed[i];
                    if in_string {
                        if escaped {
                            current_str.push(c);
                            escaped = false;
                        } else if c == b'\\' {
                            escaped = true;
                        } else if c == b')' {
                            in_string = false;
                            let s = String::from_utf8_lossy(&current_str);
                            text_content.push_str(&s);
                            current_str.clear();
                        } else {
                            current_str.push(c);
                        }
                    } else if c == b'(' {
                        in_string = true;
                    } else if c == b'\n' || c == b'\r' {
                        text_content.push('\n');
                    }
                    i += 1;
                }
                text_content.push('\n');
            }
            pos = actual_end + 9;
        } else {
            break;
        }
    }

    if text_content.trim().is_empty() {
        None
    } else {
        let cleaned: Vec<String> = text_content
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Some(cleaned.join("\n"))
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::ZlibEncoder};
    use std::io::Write;

    fn pdf_with_stream(content: &[u8]) -> Vec<u8> {
        let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
        enc.write_all(content).unwrap();
        let mut pdf = b"stream\n".to_vec();
        pdf.extend(enc.finish().unwrap());
        pdf.extend(b"endstream");
        pdf
    }

    #[test]
    fn extracts_text_from_flate_stream() {
        let pdf = pdf_with_stream(b"BT (Hello) Tj\n(World) Tj ET");
        assert_eq!(extract_pdf_text(&pdf).as_deref(), Some("Hello\nWorld"));
    }

    #[test]
    fn inflation_is_capped_against_decompression_bombs() {
        let bomb = b"(x)".repeat(12 * 1024 * 1024);
        let pdf = pdf_with_stream(&bomb);
        assert!(pdf.len() < 1024 * 1024, "the fixture is a small file");
        let text = extract_pdf_text(&pdf).unwrap();
        assert!(text.len() as u64 <= MAX_INFLATED_BYTES);
    }
}
