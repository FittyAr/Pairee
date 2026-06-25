use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, Row, Table},
};

pub fn render_yazi_popup(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let (title, rows) = match popup {
        PopupType::YaziSortPopup => {
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
            (" Sort modes (Yazi style) ", vec![row1, row2, row3])
        }
        PopupType::YaziViewPopup => {
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
            (" View modes (Yazi style) ", vec![row1, row2, row3])
        }
        _ => return false,
    };

    let panel_height = 4;
    let area = Rect::new(
        0,
        size.height.saturating_sub(panel_height),
        size.width,
        panel_height,
    );
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.popup_bg)));

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .block(block);

    f.render_widget(table, area);
    true
}
