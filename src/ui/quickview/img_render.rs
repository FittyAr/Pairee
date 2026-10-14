use ratatui::{Frame, layout::Rect, widgets::Block};

pub fn render_quick_view_image(
    f: &mut Frame,
    area: Rect,
    img: &std::sync::Arc<image::DynamicImage>,
    block: Block,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }
    f.render_widget(block, area);
    crate::ui::half_block_image::draw(f, inner, img, theme.panel_bg);
}
