use crate::app::state::{AppState, PopupType, Screen};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

pub fn render_screens_menu(
    f: &mut Frame,
    popup: &PopupType,
    state: &AppState,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    if let PopupType::ScreensMenu { cursor_idx, .. } = popup {
        let mut items = Vec::new();
        let mut max_width = 20;
        for (i, screen) in state.screens.iter().enumerate() {
            let active_marker = if i == state.active_screen_idx {
                "*"
            } else {
                " "
            };

            let name = match screen {
                Screen::Panels => "Panels".to_string(),
                Screen::Editor(ed) => format!("Edit: {}", ed.path.display()),
                Screen::Viewer(vw) => format!("View: {}", vw.path.display()),
                Screen::Terminal(ts) => format!("Term: {}", ts.command),
            };

            let text = format!("{} {} {}", active_marker, i + 1, name);
            if text.len() > max_width {
                max_width = text.len();
            }
            items.push(ListItem::new(Line::from(vec![Span::raw(text)])));
        }

        let menu_width = (max_width as u16 + 4).min(size.width.saturating_sub(4));
        let menu_height = (items.len() as u16 + 2)
            .clamp(5, 20)
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
            .title(" Screens ")
            .style(Style::default().bg(parse_color(&theme.popup_bg)));

        let mut list_state = ListState::default();
        list_state.select(Some(*cursor_idx));

        let list = List::new(items)
            .block(block)
            .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black))
            .style(Style::default().fg(parse_color(&theme.popup_fg)));

        f.render_stateful_widget(list, area, &mut list_state);

        true
    } else {
        false
    }
}
