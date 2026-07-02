use crate::app::state::PopupType;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

pub fn render_dev_select(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::SelectDevPlugin {
        options,
        cursor_idx,
        ..
    } = popup
    {
        let mut items = Vec::new();
        let mut max_width = 30;

        for (display_name, _) in options {
            if display_name.len() > max_width {
                max_width = display_name.len();
            }
            items.push(ListItem::new(Line::from(vec![Span::raw(
                display_name.clone(),
            )])));
        }

        let menu_width = (max_width as u16 + 6).min(size.width.saturating_sub(4));
        let menu_height = (items.len() as u16 + 2)
            .max(5)
            .min(15)
            .min(size.height.saturating_sub(4));

        let area = Rect {
            x: size.width.saturating_sub(menu_width) / 2,
            y: size.height.saturating_sub(menu_height) / 2,
            width: menu_width,
            height: menu_height,
        };

        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Select Active Development Plugin ")
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let mut list_state = ListState::default();
        list_state.select(Some(*cursor_idx));

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(parse_color(&theme.popup_fg)));

        f.render_stateful_widget(list, area, &mut list_state);

        true
    } else {
        false
    }
}
