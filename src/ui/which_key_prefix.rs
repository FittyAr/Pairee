//! Non-modal prefix HUD: remaining chords while a `keybinds` sequence is ongoing.
//!
//! This is display-only — the next key still goes to the same resolver.

use crate::app::context::AppContext;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame, context: &AppContext, area: Rect) {
    if !context.resolver.is_ongoing() {
        return;
    }
    let items = context.resolver.prefix_completions();
    let prefix = context.resolver.ongoing_prefix_display();
    if prefix.is_empty() && items.is_empty() {
        return;
    }

    let title = format!(" {} {prefix} ", t("which_key_prefix"));
    let rows: Vec<Line> = if items.is_empty() {
        vec![Line::from(Span::raw("…"))]
    } else {
        items
            .iter()
            .take(10)
            .map(|(rest, label, _)| {
                Line::from(vec![
                    Span::styled(
                        format!(" {rest:<10} "),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(label.as_str()),
                ])
            })
            .collect()
    };

    let height = (rows.len() as u16)
        .saturating_add(2)
        .min(12)
        .min(area.height.saturating_sub(1));
    let width = area.width.min(48).max(20.min(area.width));
    if height < 3 || width < 12 {
        return;
    }
    let x = area.x + area.width.saturating_sub(width).saturating_sub(1);
    let y = area.y + area.height.saturating_sub(height).saturating_sub(1);
    let rect = Rect::new(x, y, width, height);

    let theme = &context.config.theme;
    f.render_widget(Clear, rect);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .style(
            Style::default()
                .fg(parse_color(&theme.popup_fg))
                .bg(parse_color(&theme.popup_bg)),
        );
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    f.render_widget(Paragraph::new(rows), inner);
}
