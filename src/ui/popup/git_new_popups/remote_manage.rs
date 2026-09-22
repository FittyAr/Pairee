use crate::app::state::popup::{GitRemoteAddState, GitRemoteManageState};
use crate::config::theme::Theme;
use crate::ui::popup::centered_rect;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render_remote_manage(
    f: &mut Frame,
    state: &GitRemoteManageState,
    theme: &Theme,
    size: Rect,
) -> bool {
    let area = centered_rect(70, 60, size);
    f.render_widget(Clear, area);

    let border_style = Style::default().fg(Color::Cyan);
    let title = crate::config::localization::t("git_remote_manage_title");
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
            Constraint::Min(3),    // Remote list
            Constraint::Length(1), // Hint bar
        ])
        .split(inner);

    if state.remotes.is_empty() {
        let empty_msg = crate::config::localization::t("git_remote_no_remotes");
        let p = Paragraph::new(empty_msg)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(p, chunks[0]);
    } else {
        let lines: Vec<Line> = state
            .remotes
            .iter()
            .enumerate()
            .map(|(i, remote)| {
                let is_selected = i == state.selected_idx;
                let bg = if is_selected {
                    parse_color(&theme.selection_bg)
                } else {
                    parse_color(&theme.popup_bg)
                };
                let fg = if is_selected {
                    parse_color(&theme.selection_fg)
                } else {
                    parse_color(&theme.popup_fg)
                };
                let url = remote.url.as_deref().unwrap_or("-");
                Line::from(vec![
                    Span::styled(
                        format!(" {:<16} ", remote.name),
                        Style::default()
                            .fg(Color::Yellow)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" {} ", url), Style::default().fg(fg).bg(bg)),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(lines), chunks[0]);
    }

    let hint = crate::config::localization::t("git_remote_manage_hint");
    let hint_p = Paragraph::new(hint)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint_p, chunks[1]);

    true
}

pub fn render_remote_add(
    f: &mut Frame,
    state: &GitRemoteAddState,
    theme: &Theme,
    size: Rect,
) -> bool {
    let area = centered_rect(65, 38, size);
    f.render_widget(Clear, area);

    let border_style = Style::default().fg(Color::Cyan);
    let title = crate::config::localization::t("git_remote_add_title");
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
            Constraint::Length(1), // Name label
            Constraint::Length(3), // Name input box
            Constraint::Length(1), // URL label
            Constraint::Length(3), // URL input box
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Hint bar
        ])
        .split(inner);

    let name_label = crate::config::localization::t("git_remote_name_label");
    f.render_widget(
        Paragraph::new(name_label).style(Style::default().fg(parse_color(&theme.popup_fg))),
        chunks[0],
    );

    let name_border_color = if !state.focus_url {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(name_border_color));
    f.render_widget(
        Paragraph::new(state.name_input.as_str()).block(name_block),
        chunks[1],
    );

    let url_label = crate::config::localization::t("git_remote_url_label");
    f.render_widget(
        Paragraph::new(url_label).style(Style::default().fg(parse_color(&theme.popup_fg))),
        chunks[2],
    );

    let url_border_color = if state.focus_url {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let url_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(url_border_color));
    f.render_widget(
        Paragraph::new(state.url_input.as_str()).block(url_block),
        chunks[3],
    );

    let hint = crate::config::localization::t("git_remote_add_hint");
    let hint_p = Paragraph::new(hint)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(hint_p, chunks[5]);

    true
}
