//! Below a minimum size the layout cannot fit panels, the command line and
//! the key bar: the screen then only asks for a bigger terminal.

use crate::config::localization::t;
use crate::ui::wrap::wrap_text;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

/// Smallest terminal the normal layout is drawn in.
pub const MIN_WIDTH: u16 = 24;
pub const MIN_HEIGHT: u16 = 8;

pub fn is_too_small(area: Rect) -> bool {
    area.width < MIN_WIDTH || area.height < MIN_HEIGHT
}

/// Draws the "terminal too small" message (wrapped and centered as far as
/// it fits).
pub fn render(f: &mut Frame) {
    let area = f.area();
    let text = t("terminal_too_small")
        .replacen("{}", &area.width.to_string(), 1)
        .replacen("{}", &area.height.to_string(), 1)
        .replacen("{}", &MIN_WIDTH.to_string(), 1)
        .replacen("{}", &MIN_HEIGHT.to_string(), 1);
    let lines: Vec<Line> = wrap_text(&text, usize::from(area.width))
        .into_iter()
        .map(Line::from)
        .collect();
    let height = u16::try_from(lines.len())
        .unwrap_or(u16::MAX)
        .min(area.height);
    let y = area.y + (area.height - height) / 2;
    let message = Rect::new(area.x, y, area.width, height);
    f.render_widget(
        Paragraph::new(lines)
            .centered()
            .style(Style::default().add_modifier(Modifier::BOLD)),
        message,
    );
}
