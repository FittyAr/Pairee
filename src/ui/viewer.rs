use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Viewing mode for the internal file viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerMode {
    Text,
    Hex,
}

/// State for the internal F3 viewer.
#[derive(Debug, Clone)]
pub struct ViewerState {
    pub path: std::path::PathBuf,
    /// Lines of text content (used in Text mode).
    pub lines: Vec<String>,
    /// Raw bytes (used in Hex mode).
    pub raw: Vec<u8>,
    pub mode: ViewerMode,
    /// Vertical scroll offset (line or hex row index).
    pub scroll: usize,
}

impl ViewerState {
    /// Loads a file for viewing. Tries to read as UTF-8 text; falls back to hex mode on failure.
    pub fn load(path: std::path::PathBuf) -> Self {
        let raw = std::fs::read(&path).unwrap_or_default();
        let (lines, mode) = match std::str::from_utf8(&raw) {
            Ok(text) => (
                text.lines().map(|l| l.to_string()).collect(),
                ViewerMode::Text,
            ),
            Err(_) => (Vec::new(), ViewerMode::Hex),
        };
        Self {
            path,
            lines,
            raw,
            mode,
            scroll: 0,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            ViewerMode::Text => ViewerMode::Hex,
            ViewerMode::Hex => ViewerMode::Text,
        };
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = match self.mode {
            ViewerMode::Text => self.lines.len().saturating_sub(1),
            ViewerMode::Hex => (self.raw.len() / 16).saturating_sub(1),
        };
        self.scroll = (self.scroll + amount).min(max);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Public rendering entry point — satisfies the Viewer trait pattern from plan
// ─────────────────────────────────────────────────────────────────────────────

/// Renders the internal viewer into `area` according to the current mode.
pub fn render_viewer(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    theme: &crate::config::theme::Theme,
) {
    let mode_label = match state.mode {
        ViewerMode::Text => t("view_text_mode"),
        ViewerMode::Hex => t("view_hex_mode"),
    };
    let file_name = state
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    
    let title = t("viewer_title_bar")
        .replacen("{}", &mode_label, 1)
        .replacen("{}", &file_name, 1);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(parse_color(&theme.header_fg))
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    match state.mode {
        ViewerMode::Text => render_text(f, area, state, block, theme),
        ViewerMode::Hex => render_hex(f, area, state, block, theme),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Text renderer
// ─────────────────────────────────────────────────────────────────────────────

fn render_text(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    block: Block,
    theme: &crate::config::theme::Theme,
) {
    let height = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = state
        .lines
        .iter()
        .skip(state.scroll)
        .take(height)
        .map(|l| Line::from(Span::raw(l.clone())))
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.popup_fg)))
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Hex renderer
// ─────────────────────────────────────────────────────────────────────────────

fn render_hex(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    block: Block,
    theme: &crate::config::theme::Theme,
) {
    let height = area.height.saturating_sub(2) as usize;
    let bytes_per_row = 16usize;
    let start_byte = state.scroll * bytes_per_row;

    let lines: Vec<Line> = (0..height)
        .map(|row_offset| {
            let offset = start_byte + row_offset * bytes_per_row;
            if offset >= state.raw.len() {
                return Line::from(Span::raw(""));
            }
            let chunk = &state.raw[offset..(offset + bytes_per_row).min(state.raw.len())];

            // Hex portion
            let hex_str: String = chunk
                .iter()
                .map(|b| format!("{:02X} ", b))
                .collect::<Vec<_>>()
                .join("");
            // Pad to fixed width (16 * 3 = 48 chars)
            let hex_padded = format!("{:<48}", hex_str);

            // ASCII portion
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

            let line_str = format!("{:08X}  {}  {}", offset, hex_padded, ascii_str);
            Line::from(Span::raw(line_str))
        })
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.popup_fg)));
    f.render_widget(para, area);
}
