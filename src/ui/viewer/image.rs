use super::state::ViewerState;
use crate::config::localization::t;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Paragraph},
};

pub(crate) fn render_image(
    f: &mut Frame,
    area: Rect,
    state: &ViewerState,
    block: Block,
    theme: &crate::config::theme::Theme,
    show_scrollbar: bool,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let inner_area = block.inner(area);
    let inner_h = inner_area.height;

    if inner_area.width == 0 || inner_h == 0 {
        return;
    }

    let img = match &state.image_data {
        Some(i) => i,
        None => {
            let para = Paragraph::new(vec![Line::from(t("view_image_error"))])
                .block(block)
                .style(Style::default().fg(parse_color(&theme.panel_fg)));
            f.render_widget(para, area);
            return;
        }
    };

    f.render_widget(block, area);
    let rows = crate::ui::half_block_image::draw(
        f,
        inner_area,
        img,
        state.scroll,
        parse_color(&theme.panel_bg),
    );
    if show_scrollbar && rows > inner_h as usize {
        scrollbar::render_vertical_right(
            f,
            inner_area,
            rows,
            inner_h as usize,
            state.scroll,
            theme,
            ScrollbarSurface::Panel,
            scrollbar,
            ScrollTargetId::Viewer,
        );
    }
}
