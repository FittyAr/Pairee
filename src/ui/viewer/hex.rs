use super::state::{HEX_ROW_BYTES, ViewerState};
use ratatui::text::{Line, Span};

pub(crate) fn format_hex_row(offset: usize, chunk: &[u8]) -> String {
    let hex_str: String = chunk
        .iter()
        .map(|b| format!("{:02X} ", b))
        .collect::<Vec<_>>()
        .join("");
    let hex_padded = format!("{:<48}", hex_str);
    let ascii_str: String = chunk
        .iter()
        .map(|&b| {
            if (0x20..=0x7e).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    format!("{:08X}  {}  {}", offset, hex_padded, ascii_str)
}

/// The `height` visible hex rows, read through the document's block cache.
pub(crate) fn hex_lines(state: &ViewerState, height: usize) -> Vec<Line<'static>> {
    let start_byte = state.scroll * HEX_ROW_BYTES;
    state
        .doc
        .bytes(start_byte as u64, height * HEX_ROW_BYTES)
        .chunks(HEX_ROW_BYTES)
        .enumerate()
        .map(|(row, chunk)| {
            let offset = start_byte + row * HEX_ROW_BYTES;
            Line::from(Span::raw(format_hex_row(offset, chunk)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::format_hex_row;

    #[test]
    fn format_hex_row_shows_offset_hex_and_ascii() {
        let row = format_hex_row(0x10, &[0x41, 0x00, 0x7e]);
        assert!(row.starts_with("00000010"), "row={row}");
        assert!(row.contains("41 "), "row={row}");
        assert!(row.contains("00 "), "row={row}");
        assert!(row.contains("A.~"), "row={row}");
    }
}
