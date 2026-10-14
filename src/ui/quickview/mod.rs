mod img_render;
mod plugin;

pub use plugin::install_layout_solver;

pub use img_render::render_quick_view_image;
pub use plugin::render_plugin_widget;

use crate::config::localization::t;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface, ScrollbarUiState};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// The quick view takes the place of the passive panel
/// (`ui::screens::render_side`), so its dialog entry paints nothing itself.
impl crate::ui::popup::view::DialogView for crate::app::state::popup::QuickViewDialog {
    fn render(&self, _f: &mut Frame, _cx: &crate::ui::popup::view::ViewCtx<'_>) {}
}

/// Renders a quick-view panel showing the text or image content of a file, or a custom plugin widget.
/// Called when `state.panels.quick_view_active` is true; renders into the passive panel area.
///
/// - Scrolls vertically via `scroll` offset.
/// - Non-UTF-8 files show a binary notice.
pub fn draw_quick_view(
    f: &mut Frame,
    area: Rect,
    qv: &crate::app::state::popup::QuickViewDialog,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    scrollbar: Option<&ScrollbarUiState>,
    tab_size: usize,
) {
    let (path, content, scroll) = (&qv.path, qv.content.as_slice(), qv.scroll);
    let (image_data, plugin_widget) = (qv.image_data.as_ref(), qv.plugin.as_ref());
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "?".to_string());

    let title = t("quickview_title").replacen("{}", &file_name, 1);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border))
        .title(Line::from(Span::styled(
            title,
            Style::default()
                .fg(theme.header_fg)
                .add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(theme.panel_bg));

    if let Some(widget) = plugin_widget {
        render_plugin_widget(f, area, &widget.widget, block, theme, scroll, scrollbar);
    } else if let Some(img) = image_data {
        render_quick_view_image(f, area, img, block, theme);
    } else {
        let visible_height = area.height.saturating_sub(2) as usize;
        let lines: Vec<Line> = content
            .iter()
            .skip(scroll)
            .take(visible_height)
            // ratatui draws nothing for a tab: expand it like the viewer.
            .map(|l| Line::from(Span::raw(crate::ui::text_width::expand_tabs(l, tab_size))))
            .collect();

        let para = Paragraph::new(lines)
            .block(block)
            .style(Style::default().fg(theme.panel_fg))
            .wrap(Wrap { trim: false });

        f.render_widget(para, area);

        scrollbar::render_vertical_inside_block(
            f,
            area,
            ScrollView {
                content_len: content.len(),
                viewport_len: visible_height,
                offset: scroll,
            },
            theme,
            ScrollTarget {
                surface: ScrollbarSurface::Panel,
                hits: scrollbar,
                id: ScrollTargetId::QuickView,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn tabs_keep_their_indentation() {
        let qv = crate::app::state::popup::QuickViewDialog {
            path: "a.rs".into(),
            content: vec!["\tx".into()],
            scroll: 0,
            image_data: None,
            plugin: None,
        };
        let theme = crate::ui::resolved_theme::ResolvedTheme::default();
        let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
        terminal
            .draw(|f| draw_quick_view(f, f.area(), &qv, &theme, None, 4))
            .unwrap();
        let buf = terminal.backend().buffer();
        // Inside the frame: four columns of indentation, then the text.
        assert_eq!(buf[(5, 1)].symbol(), "x");
    }
}
