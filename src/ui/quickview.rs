use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Renders a quick-view panel showing the text content of a file.
/// Called when `state.quick_view_active` is true; renders into the passive panel area.
///
/// - Scrolls vertically via `scroll` offset.
/// - Non-UTF-8 files show a binary notice.
pub fn draw_quick_view(
    f: &mut Frame,
    area: Rect,
    path: &std::path::Path,
    content: &[String],
    scroll: usize,
    theme: &crate::config::theme::Theme,
) {
    let title = format!(
        " Quick View: {} ",
        path.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "?".to_string())
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(
            ratatui::widgets::block::Title::from(
                Span::styled(title, Style::default().fg(parse_color(&theme.header_fg)).add_modifier(Modifier::BOLD))
            )
        )
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    let visible_height = area.height.saturating_sub(2) as usize;
    let lines: Vec<Line> = content
        .iter()
        .skip(scroll)
        .take(visible_height)
        .map(|l| Line::from(Span::raw(l.clone())))
        .collect();

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(parse_color(&theme.panel_fg)))
        .wrap(Wrap { trim: false });

    f.render_widget(para, area);
}

/// Loads a file into lines for quick view. Returns a Vec of lines or a single binary notice.
pub fn load_quick_view_content(path: &std::path::Path) -> Vec<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => text.lines().map(|l| l.to_string()).collect(),
        Err(_) => vec!["[Binary file — cannot preview]".to_string()],
    }
}
