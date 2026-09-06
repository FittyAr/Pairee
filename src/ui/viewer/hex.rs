use super::state::ViewerState;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

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

pub(crate) fn render_hex(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    block: Block,
    theme: &crate::config::theme::Theme,
    show_scrollbar: bool,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let height = area.height.saturating_sub(2) as usize;
    let bytes_per_row = 16usize;
    let start_byte = state.scroll * bytes_per_row;
    let total_rows = state.raw.len().div_ceil(bytes_per_row).max(1);

    let lines: Vec<Line> = (0..height)
        .map(|row_offset| {
            let offset = start_byte + row_offset * bytes_per_row;
            if offset >= state.raw.len() {
                return Line::from(Span::raw(""));
            }
            let chunk = &state.raw[offset..(offset + bytes_per_row).min(state.raw.len())];
            Line::from(Span::raw(format_hex_row(offset, chunk)))
        })
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.panel_fg)));
    f.render_widget(para, area);

    if show_scrollbar {
        scrollbar::render_vertical_inside_block(
            f,
            area,
            total_rows,
            height,
            state.scroll,
            theme,
            ScrollbarSurface::Panel,
            scrollbar,
            ScrollTargetId::Viewer,
        );
    }
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
