use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use image::GenericImageView;
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
    Image,
}

/// State for the internal F3 viewer.
#[derive(Debug, Clone)]
pub struct ViewerState {
    pub path: std::path::PathBuf,
    /// Lines of text content (used in Text mode).
    pub lines: Vec<String>,
    /// Raw bytes (used in Hex mode).
    pub raw: Vec<u8>,
    /// Loaded image data if applicable.
    pub image_data: Option<image::DynamicImage>,
    pub is_image: bool,
    pub is_text: bool,
    pub mode: ViewerMode,
    /// Vertical scroll offset (line, hex row index, or image character row).
    pub scroll: usize,
    /// Last search query
    pub last_search: Option<String>,
}

impl ViewerState {
    /// Loads a file for viewing. Tries to read as UTF-8 text; falls back to hex mode on failure.
    pub fn load(path: std::path::PathBuf) -> Self {
        let raw = std::fs::read(&path).unwrap_or_default();

        let is_image_ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                let ext_lower = ext.to_lowercase();
                matches!(
                    ext_lower.as_str(),
                    "png"
                        | "jpg"
                        | "jpeg"
                        | "bmp"
                        | "gif"
                        | "webp"
                        | "tif"
                        | "tiff"
                        | "ico"
                        | "tga"
                )
            })
            .unwrap_or(false);

        let mut image_data = None;
        let mut mode = ViewerMode::Hex;

        if is_image_ext {
            if let Ok(img) = image::open(&path) {
                image_data = Some(img);
                mode = ViewerMode::Image;
            }
        }

        let is_image = image_data.is_some();
        let is_text = std::str::from_utf8(&raw).is_ok();

        let lines = if is_text {
            std::str::from_utf8(&raw)
                .unwrap_or_default()
                .lines()
                .map(|l| l.to_string())
                .collect()
        } else {
            String::from_utf8_lossy(&raw)
                .lines()
                .map(|l| l.to_string())
                .collect()
        };

        if !is_image {
            if is_text {
                mode = ViewerMode::Text;
            } else {
                mode = ViewerMode::Hex;
            }
        }

        Self {
            path,
            lines,
            raw,
            image_data,
            is_image,
            is_text,
            mode,
            scroll: 0,
            last_search: None,
        }
    }

    pub fn toggle_mode(&mut self) {
        if self.is_image && !self.is_text {
            // Image files switch between Image and Hex
            self.mode = match self.mode {
                ViewerMode::Image => ViewerMode::Hex,
                _ => ViewerMode::Image,
            };
        } else if self.is_text && !self.is_image {
            // Text/code files switch between Text and Hex
            self.mode = match self.mode {
                ViewerMode::Text => ViewerMode::Hex,
                _ => ViewerMode::Text,
            };
        } else {
            // Unfiltered / other files switch between all 3 modes
            self.mode = match self.mode {
                ViewerMode::Text => ViewerMode::Hex,
                ViewerMode::Hex => ViewerMode::Image,
                ViewerMode::Image => ViewerMode::Text,
            };
        }
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        let max = match self.mode {
            ViewerMode::Text => self.lines.len().saturating_sub(1),
            ViewerMode::Hex => (self.raw.len() / 16).saturating_sub(1),
            ViewerMode::Image => {
                if let Some(ref img) = self.image_data {
                    (img.height() as usize / 2).saturating_sub(1)
                } else {
                    0
                }
            }
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
        ViewerMode::Image => t("view_image_mode"),
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
        .border_style(Style::default().fg(parse_color(&theme.panel_border)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(parse_color(&theme.header_fg))
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    match state.mode {
        ViewerMode::Text => render_text(f, area, state, block, theme),
        ViewerMode::Hex => render_hex(f, area, state, block, theme),
        ViewerMode::Image => render_image(f, area, state, block, theme),
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
        .style(Style::default().fg(parse_color(&theme.panel_fg)))
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
        .style(Style::default().fg(parse_color(&theme.panel_fg)));
    f.render_widget(para, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Image renderer
// ─────────────────────────────────────────────────────────────────────────────

fn render_image(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    block: Block,
    theme: &crate::config::theme::Theme,
) {
    let inner_area = block.inner(area);
    let inner_w = inner_area.width;
    let inner_h = inner_area.height;

    if inner_w == 0 || inner_h == 0 {
        return;
    }

    let img = match &state.image_data {
        Some(i) => i,
        None => {
            let para = Paragraph::new(vec![Line::from("Error loading image")])
                .block(block)
                .style(Style::default().fg(parse_color(&theme.panel_fg)));
            f.render_widget(para, area);
            return;
        }
    };

    let img_w = img.width();
    let img_h = img.height();

    // Virtual height of terminal canvas: each cell has 2 vertical pixels.
    let canvas_w = inner_w as u32;
    let canvas_h = inner_h as u32 * 2;

    // Preserve aspect ratio
    let r = img_w as f64 / img_h as f64;
    let (mut dw, mut dh) = if (canvas_w as f64 / r) <= canvas_h as f64 {
        let w = canvas_w;
        let h = (w as f64 / r) as u32;
        (w, h)
    } else {
        let h = canvas_h;
        let w = (h as f64 * r) as u32;
        (w, h)
    };

    if dw == 0 {
        dw = 1;
    }
    if dh == 0 {
        dh = 1;
    }

    // Resize image using fast filter
    let resized = img.resize_exact(dw, dh, image::imageops::FilterType::Nearest);

    let cols = dw as usize;
    let rows = (dh as usize + 1) / 2;
    let scroll_offset = state.scroll;

    // Center layout calculations (relative to inner_area)
    let start_x = inner_area.x + ((inner_w - dw as u16) / 2);
    let start_y = inner_area.y + ((inner_h - rows as u16) / 2);

    f.render_widget(block, area);

    let buf = f.buffer_mut();

    for r_y in 0..rows {
        let target_y = start_y as i32 + r_y as i32 - scroll_offset as i32;
        if target_y < inner_area.y as i32 || target_y >= (inner_area.y + inner_h) as i32 {
            continue;
        }

        for r_x in 0..cols {
            let target_x = start_x + r_x as u16;
            if target_x >= inner_area.x + inner_w {
                continue;
            }

            let py_top = 2 * r_y;
            let py_bottom = 2 * r_y + 1;

            let pixel_top = resized.get_pixel(r_x as u32, py_top as u32);
            let color_top = ratatui::style::Color::Rgb(pixel_top[0], pixel_top[1], pixel_top[2]);

            let cell = buf.get_mut(target_x, target_y as u16);
            if py_bottom < dh as usize {
                let pixel_bottom = resized.get_pixel(r_x as u32, py_bottom as u32);
                let color_bottom =
                    ratatui::style::Color::Rgb(pixel_bottom[0], pixel_bottom[1], pixel_bottom[2]);
                cell.set_char('▄');
                cell.set_fg(color_bottom);
                cell.set_bg(color_top);
            } else {
                cell.set_char('▀');
                cell.set_fg(color_top);
                cell.set_bg(parse_color(&theme.panel_bg));
            }
        }
    }
}
