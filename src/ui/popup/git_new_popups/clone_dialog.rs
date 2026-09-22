use crate::app::state::popup::GitClonePromptState;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_clone(f: &mut Frame, state: &GitClonePromptState, theme: &Theme, size: Rect) -> bool {
    let area = centered_rect(65, 38, size);
    f.render_widget(Clear, area);

    let border_style = Style::default().fg(Color::Cyan);
    let title = t("git_clone_title");
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // URL label
            Constraint::Length(3), // URL input box
            Constraint::Length(1), // Dir label
            Constraint::Length(3), // Dir input box
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint bar
        ])
        .split(inner);

    let url_label = t("git_clone_url_label");
    f.render_widget(
        Paragraph::new(url_label).style(Style::default().fg(parse_color(&theme.popup_fg))),
        chunks[0],
    );

    let url_border_color = if !state.focus_dir {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(url_border_color));
    f.render_widget(
        Paragraph::new(state.url_input.as_str()).block(url_block),
        chunks[1],
    );

    let dir_label = t("git_clone_dir_label");
    f.render_widget(
        Paragraph::new(dir_label).style(Style::default().fg(parse_color(&theme.popup_fg))),
        chunks[2],
    );

    let dir_border_color = if state.focus_dir {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let dir_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dir_border_color));
    f.render_widget(
        Paragraph::new(state.dir_input.as_str()).block(dir_block),
        chunks[3],
    );

    let hint = t("git_clone_hint");
    let hint_p = Paragraph::new(hint)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint_p, chunks[5]);

    true
}
