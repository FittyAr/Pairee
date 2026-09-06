mod hex;
mod image;
mod state;
mod text;

pub use state::{ViewerMode, ViewerState};

use crate::config::localization::t;
use crate::ui::scrollbar::ScrollbarUiState;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders},
};

/// Renders the internal viewer into `area` according to the current mode.
pub fn render_viewer(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    theme: &crate::config::theme::Theme,
    active_popup: Option<&crate::app::state::PopupType>,
    show_scrollbar: bool,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let mode_label = match state.mode {
        ViewerMode::Text => t("view_text_mode"),
        ViewerMode::Hex => t("view_hex_mode"),
        ViewerMode::Image => t("view_image_mode"),
    };
    let file_name = state
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let title = t("viewer_title_bar")
        .replacen("{}", &mode_label, 1)
        .replacen("{}", &file_name, 1);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.panel_border)))
        .title(Span::styled(
            title,
            Style::default()
                .fg(parse_color(&theme.header_fg))
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    match state.mode {
        ViewerMode::Text => text::render_text(
            f,
            area,
            state,
            block,
            theme,
            active_popup,
            show_scrollbar,
            scrollbar,
        ),
        ViewerMode::Hex => hex::render_hex(f, area, state, block, theme, show_scrollbar, scrollbar),
        ViewerMode::Image => {
            image::render_image(f, area, state, block, theme, show_scrollbar, scrollbar)
        }
    }
}
