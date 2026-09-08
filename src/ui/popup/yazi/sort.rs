use crate::ui::theme_apply::parse_color;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Row},
};

pub fn build_sort_rows(theme: &crate::config::theme::Theme) -> Vec<Row<'static>> {
    let row1 = Row::new(vec![
        Cell::from(Line::from(vec![
            Span::styled(
                " n ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(" Name", Style::default().fg(parse_color(&theme.popup_fg))),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " e ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Extension",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " s ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(" Size", Style::default().fg(parse_color(&theme.popup_fg))),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " w ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Write time",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
    ]);
    let row2 = Row::new(vec![
        Cell::from(Line::from(vec![
            Span::styled(
                " c ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Creation time",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " a ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Access time",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " d ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Description",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " o ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(" Owner", Style::default().fg(parse_color(&theme.popup_fg))),
        ])),
    ]);
    let row3 = Row::new(vec![
        Cell::from(Line::from(vec![
            Span::styled(
                " u ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Unsorted",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " r ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Reverse order",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(""),
        Cell::from(""),
    ]);
    vec![row1, row2, row3]
}
