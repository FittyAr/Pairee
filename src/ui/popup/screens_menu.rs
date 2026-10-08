use super::kit::{ListPopup, Scroll, list_area, popup_fg};
use crate::app::state::{AppState, PopupType, Screen};
use ratatui::{Frame, layout::Rect, style::Color};

pub fn render_screens_menu(
    f: &mut Frame,
    popup: &PopupType,
    state: &AppState,
    theme: &crate::config::theme::Theme,
    size: Rect,
) -> bool {
    let PopupType::ScreensMenu { cursor_idx, .. } = popup else {
        return false;
    };
    let rows: Vec<(String, _)> = state
        .screens
        .iter()
        .enumerate()
        .map(|(i, screen)| {
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
            (
                format!("{} {} {}", active_marker, i + 1, name),
                popup_fg(theme),
            )
        })
        .collect();
    let width = rows.iter().map(|(text, _)| text.len()).max().unwrap_or(0);
    ListPopup {
        area: list_area(size, width.max(20), rows.len(), 20),
        title: " Screens ".to_string(),
        border: Color::Yellow,
        empty: None,
        header: Vec::new(),
        rows,
        cursor: *cursor_idx,
        scroll: Scroll::Centered,
        hint: None,
        scrollbar: None,
    }
    .render(f, theme);
    true
}
