mod img_render;
mod loader;
mod plugin;

pub use img_render::render_quick_view_image;
pub use loader::load_quick_view_content;
pub use plugin::render_plugin_widget;

use crate::app::state::types::PluginWidget;
use crate::config::localization::t;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Renders a quick-view panel showing the text or image content of a file, or a custom plugin widget.
/// Called when `state.panels.quick_view_active` is true; renders into the passive panel area.
///
/// - Scrolls vertically via `scroll` offset.
/// - Non-UTF-8 files show a binary notice.
pub fn draw_quick_view(
    f: &mut Frame,
    area: Rect,
    path: &std::path::Path,
    content: &[String],
    scroll: usize,
    theme: &crate::config::theme::Theme,
    image_data: &Option<image::DynamicImage>,
    plugin_widget: &Option<PluginWidget>,
    scrollbar: Option<&ScrollbarUiState>,
) {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "?".to_string());

    let title = t("quickview_title").replacen("{}", &file_name, 1);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .title(Line::from(Span::styled(
            title,
            Style::default()
                .fg(parse_color(&theme.header_fg))
                .add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    if let Some(widget) = plugin_widget {
        render_plugin_widget(f, area, widget, block, theme, scroll, scrollbar);
    } else if let Some(img) = image_data {
        render_quick_view_image(f, area, img, block, theme, scroll);
    } else {
        let visible_height = area.height.saturating_sub(2) as usize;
        let lines: Vec<Line> = content
            .iter()
            .skip(scroll)
            .take(visible_height)
            .map(|l| Line::from(Span::raw(l.clone())))
            .collect();

        let para = Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(parse_color(&theme.panel_fg)))
            .wrap(Wrap { trim: false });

        f.render_widget(para, area);

        scrollbar::render_vertical_inside_block(
            f,
            area,
            content.len(),
            visible_height,
            scroll,
            theme,
            ScrollbarSurface::Panel,
            scrollbar,
            ScrollTargetId::QuickView,
        );
    }
}
