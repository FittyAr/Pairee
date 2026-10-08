use crate::ui::theme_apply::parse_color;
use ratatui::{Frame, layout::Rect, widgets::Block};

pub fn render_quick_view_image(
    f: &mut Frame,
    area: Rect,
    img: &image::DynamicImage,
    block: Block,
    theme: &crate::config::theme::Theme,
    scroll_offset: usize,
) {
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    f.render_widget(block, area);
    crate::ui::half_block_image::draw(f, inner, img, scroll_offset, parse_color(&theme.panel_bg));
}
