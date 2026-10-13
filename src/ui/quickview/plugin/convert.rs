//! Plugin widget data ([`crate::app::state::popup::plugin_widget`]) to
//! ratatui styles, text, borders and constraints.

use crate::app::state::popup::plugin_widget::{
    WAlign, WBorder, WBorderType, WConstraint, WLine, WSpan, WStyle, WText, WTextBody,
};
use crate::ui::theme_apply::parse_color;
use ansi_to_tui::IntoText as _;
use ratatui::{
    layout::{Alignment, Constraint},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders},
};

fn color(name: &str) -> Color {
    // `parse_color` gives `Reset` for unknown names, which is also what
    // `:reset()` stores.
    parse_color(name)
}

pub fn style(style: &WStyle) -> Style {
    let mut out = Style::default()
        .add_modifier(Modifier::from_bits_truncate(style.add))
        .remove_modifier(Modifier::from_bits_truncate(style.sub));
    if let Some(fg) = &style.fg {
        out = out.fg(color(fg));
    }
    if let Some(bg) = &style.bg {
        out = out.bg(color(bg));
    }
    out
}

pub fn alignment(align: WAlign) -> Alignment {
    match align {
        WAlign::Left => Alignment::Left,
        WAlign::Center => Alignment::Center,
        WAlign::Right => Alignment::Right,
    }
}

pub fn span(span: &WSpan) -> Span<'static> {
    Span::styled(span.content.clone(), style(&span.style))
}

pub fn line(line: &WLine) -> Line<'static> {
    let mut out =
        Line::from(line.spans.iter().map(span).collect::<Vec<_>>()).style(style(&line.style));
    if let Some(align) = line.align {
        out = out.alignment(alignment(align));
    }
    out
}

pub fn text(text: &WText) -> Text<'static> {
    let mut out = match &text.body {
        WTextBody::Lines(lines) => Text::from(lines.iter().map(line).collect::<Vec<_>>()),
        WTextBody::Ansi(raw) => raw
            .as_bytes()
            .into_text()
            .unwrap_or_else(|_| Text::from(raw.clone())),
    };
    out = out.patch_style(style(&text.style));
    if let Some(align) = text.align {
        out = out.alignment(alignment(align));
    }
    out
}

/// The frame of `border` (its sides use the same bits as ratatui).
pub fn block(border: &WBorder) -> Block<'static> {
    let kind = match border.kind {
        WBorderType::Plain => BorderType::Plain,
        WBorderType::Rounded => BorderType::Rounded,
        WBorderType::Double => BorderType::Double,
        WBorderType::Thick => BorderType::Thick,
    };
    let mut out = Block::default()
        .borders(Borders::from_bits_truncate(border.edges))
        .border_type(kind)
        .border_style(style(&border.style));
    if let Some(title) = &border.title {
        out = out.title(line(title));
    }
    out
}

pub fn constraint(c: &WConstraint) -> Constraint {
    match *c {
        WConstraint::Min(n) => Constraint::Min(n),
        WConstraint::Max(n) => Constraint::Max(n),
        WConstraint::Length(n) => Constraint::Length(n),
        WConstraint::Percentage(n) => Constraint::Percentage(n),
        WConstraint::Ratio(a, b) => Constraint::Ratio(a, b),
        WConstraint::Fill(n) => Constraint::Fill(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::popup::plugin_widget::{edge, modifier};

    #[test]
    fn styles_and_borders_map_to_ratatui() {
        let mut s = WStyle {
            fg: Some("red".into()),
            bg: Some("#000010".into()),
            ..WStyle::default()
        };
        s.with(modifier::BOLD | modifier::ITALIC);
        let out = style(&s);
        assert_eq!(out.fg, Some(Color::Red));
        assert_eq!(out.bg, Some(Color::Rgb(0, 0, 16)));
        assert!(out.add_modifier.contains(Modifier::BOLD | Modifier::ITALIC));
        assert_eq!(Borders::from_bits_truncate(edge::ALL), Borders::ALL);
        assert_eq!(Borders::from_bits_truncate(edge::LEFT), Borders::LEFT);
    }

    #[test]
    fn ansi_text_is_decoded() {
        let t = WText {
            body: WTextBody::Ansi("\x1b[31mred\x1b[0m plain".into()),
            ..WText::default()
        };
        let out = text(&t);
        assert_eq!(out.lines.len(), 1);
        assert_eq!(out.lines[0].spans[0].style.fg, Some(Color::Red));
    }
}
