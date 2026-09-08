use crate::ui::theme_apply::parse_color;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Row},
};

pub fn build_view_rows(theme: &crate::config::theme::Theme) -> Vec<Row<'static>> {
    let row1 = Row::new(vec![
        Cell::from(Line::from(vec![
            Span::styled(
                " 1/b ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(" Brief", Style::default().fg(parse_color(&theme.popup_fg))),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " 2/m ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(" Medium", Style::default().fg(parse_color(&theme.popup_fg))),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " 3/f ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(" Full", Style::default().fg(parse_color(&theme.popup_fg))),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " 4/w ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(" Wide", Style::default().fg(parse_color(&theme.popup_fg))),
        ])),
    ]);
    let row2 = Row::new(vec![
        Cell::from(Line::from(vec![
            Span::styled(
                " 5/d ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Detailed",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " 6/x ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Descriptions",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " 7/o ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " File owners",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " 8/l ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " File links",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
    ]);
    let row3 = Row::new(vec![
        Cell::from(Line::from(vec![
            Span::styled(
                " 9/a ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Alt full",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " i ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Info panel",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(Line::from(vec![
            Span::styled(
                " q ",
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("⋄", Style::default().fg(ratatui::style::Color::DarkGray)),
            Span::styled(
                " Quick view",
                Style::default().fg(parse_color(&theme.popup_fg)),
            ),
        ])),
        Cell::from(""),
    ]);
    vec![row1, row2, row3]
}
