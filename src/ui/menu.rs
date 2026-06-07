use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

pub fn get_menu_items(menu_idx: usize) -> &'static [&'static str] {
    match menu_idx {
        0 => &[
            " Brief View ",
            " Full View ",
            " Info Panel ",
            " Tree Panel ",
            " Drive Select   Alt+F1 ",
        ],
        1 => &[
            " Help               F1 ",
            " User Menu          F2 ",
            " View               F3 ",
            " Edit               F4 ",
            " Copy               F5 ",
            " Rename/Move        F6 ",
            " Make Directory     F7 ",
            " Delete             F8 ",
            " Exit              F10 ",
        ],
        2 => &[
            " Swap Panels     Ctrl+U ",
            " Compare Directories ",
            " Directory Hotlist ",
            " Search Files ",
        ],
        3 => &[
            " Toggle Hidden   Ctrl+H ",
            " Theme: Slate ",
            " Theme: Classic Blue ",
            " Preset: Norton ",
            " Preset: Vim ",
        ],
        4 => &[
            " Brief View ",
            " Full View ",
            " Info Panel ",
            " Tree Panel ",
            " Drive Select   Alt+F2 ",
        ],
        _ => &[],
    }
}

pub fn render_menu(f: &mut Frame, area: Rect, context: &AppContext, state: &AppState) {
    let theme = &context.config.theme;

    let items = [
        "  Left  ",
        "  Files  ",
        "  Commands  ",
        "  Options  ",
        "  Right  ",
    ];

    let active_menu_idx = if let Some(PopupType::Menu {
        active_menu_idx, ..
    }) = state.active_popup
    {
        Some(active_menu_idx)
    } else {
        None
    };

    let mut spans = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let is_active = Some(i) == active_menu_idx;
        let style = if is_active {
            Style::default()
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(parse_color(&theme.panel_fg))
                .add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(*item, style));
    }

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).style(Style::default().bg(parse_color("DarkGray")));

    f.render_widget(paragraph, area);
}
