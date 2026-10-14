use super::state::ViewerState;
use crate::config::localization::t;
use ratatui::{
    Frame,
    layout::{Margin, Rect},
    text::Line,
    widgets::Paragraph,
};

/// Text under the picture: the decode error when there is no picture.
pub(crate) fn placeholder(state: &ViewerState) -> Paragraph<'static> {
    match state.image_data {
        Some(_) => Paragraph::default(),
        None => Paragraph::new(vec![Line::from(t("view_image_error"))]),
    }
}

/// Draws the picture, fitted whole inside the viewer frame at `area`.
pub(crate) fn draw_picture(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    if let Some(img) = &state.image_data {
        let inner = area.inner(Margin::new(1, 1));
        crate::ui::half_block_image::draw(f, inner, img, theme.panel_bg);
    }
}
