//! Shared building blocks for dialog renderers: the framed popup, focus
//! styles, checkbox / choice rows, button bars, separators and the
//! [`TextField`] widget. Every prompt draws these the same way, so they live
//! here once.

use crate::app::text_input::{TextField, split_at_cursor};
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect_fixed;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Clears a centered `width` × `height` area, draws a bordered block titled
/// `title` and returns the inner area.
pub fn dialog_frame(
    f: &mut Frame,
    size: Rect,
    (width, height): (u16, u16),
    title: String,
    border: Style,
    theme: &Theme,
) -> Rect {
    let area = centered_rect_fixed(width, height, size);
    f.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(title)
        .style(Style::default().bg(parse_color(&theme.popup_bg)));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// Focused / unfocused styles for dialog rows and buttons.
#[derive(Debug, Clone, Copy)]
pub struct FocusStyles {
    pub active: Style,
    pub normal: Style,
}

impl FocusStyles {
    /// Cyan-on-black focus over the theme's popup foreground.
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            active: Style::default().bg(Color::Cyan).fg(Color::Black),
            normal: Style::default().fg(parse_color(&theme.popup_fg)),
        }
    }

    /// Style of row `row` when `focus` is the focused row.
    pub fn row(&self, focus: usize, row: usize) -> Style {
        self.pick(focus == row)
    }

    pub fn pick(&self, focused: bool) -> Style {
        if focused { self.active } else { self.normal }
    }

    /// Block cursor drawn inside a focused (active-styled) text field.
    pub fn cursor(&self) -> Style {
        self.active.add_modifier(Modifier::REVERSED)
    }
}

/// `[x]` / `[ ]`.
pub fn checkbox(checked: bool) -> &'static str {
    if checked { "[x]" } else { "[ ]" }
}

/// `"[x] label"` row.
pub fn checkbox_row(checked: bool, label: &str) -> String {
    format!("{} {}", checkbox(checked), label)
}

/// Centered `[ A ]  [ B ]  ...` bar; `focused` is the index of the focused
/// button inside `labels` (if any).
pub fn button_bar<'a>(
    labels: &[String],
    focused: Option<usize>,
    styles: FocusStyles,
) -> Paragraph<'a> {
    let mut spans = Vec::with_capacity(labels.len() * 2);
    for (i, label) in labels.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(label.clone(), styles.pick(focused == Some(i))));
    }
    Paragraph::new(Line::from(spans)).alignment(Alignment::Center)
}

/// A full-width horizontal rule.
pub fn separator<'a>(width: u16, style: Style) -> Paragraph<'a> {
    Paragraph::new(ratatui::symbols::line::HORIZONTAL.repeat(width as usize)).style(style)
}

/// Spans for a [`TextField`]: the text in `style`, and when `focused` the
/// grapheme under the cursor (or a trailing space) in `cursor`.
pub fn field_spans(
    field: &TextField,
    style: Style,
    cursor: Style,
    focused: bool,
) -> Vec<Span<'static>> {
    if !focused {
        return vec![Span::styled(field.text().to_string(), style)];
    }
    let (before, at, after) = split_at_cursor(field.text(), field.cursor());
    let at = if at.is_empty() { " " } else { at };
    vec![
        Span::styled(before.to_string(), style),
        Span::styled(at.to_string(), cursor),
        Span::styled(after.to_string(), style),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(spans: &[Span]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn field_spans_mark_the_cursor() {
        let mut field = TextField::new("abc");
        field.move_left();
        let spans = field_spans(&field, Style::default(), Style::default(), true);
        assert_eq!(spans[1].content, "c");
        assert_eq!(text(&spans), "abc");
        field.move_end();
        let spans = field_spans(&field, Style::default(), Style::default(), true);
        assert_eq!(text(&spans), "abc ");
        let spans = field_spans(&field, Style::default(), Style::default(), false);
        assert_eq!(text(&spans), "abc");
    }
}
