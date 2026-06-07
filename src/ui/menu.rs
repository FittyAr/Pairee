use crate::app::context::AppContext;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn render_menu(f: &mut Frame, area: Rect, context: &AppContext) {
    let theme = &context.config.theme;

    let items = [
        "  Left  ",
        "  Files  ",
        "  Commands  ",
        "  Options  ",
        "  Right  ",
    ];

    let mut spans = Vec::new();
    for item in items {
        spans.push(Span::styled(
            item,
            Style::default()
                .fg(parse_color(&theme.panel_fg))
                .add_modifier(Modifier::BOLD),
        ));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(parse_color("DarkGray")));

    f.render_widget(paragraph, area);
}
