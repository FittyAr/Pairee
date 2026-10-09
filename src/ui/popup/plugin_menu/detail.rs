//! Building blocks shared by the plugin-manager tabs: framed panes, the
//! selected-row style and `label: value` detail lines.

use super::Pane;
use crate::config::localization::t;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Framed block of a tab with `title`.
pub(super) fn pane_block(pane: &Pane, title: String) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(pane.border_style)
        .title(title)
        .style(pane.bg_style)
}

/// Style of a list row: highlighted under the cursor, plain otherwise.
pub(super) fn row_style(pane: &Pane, selected: bool) -> Style {
    let theme = pane.theme;
    if selected {
        Style::default()
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD)
    } else {
        text_style(pane)
    }
}

/// Plain popup text.
pub(super) fn text_style(pane: &Pane) -> Style {
    Style::default().fg(parse_color(&pane.theme.popup_fg))
}

/// `label value` with the label in bold.
pub(super) fn field_line(pane: &Pane, label_key: &str, value: String) -> Line<'static> {
    let text = text_style(pane);
    Line::from(vec![
        Span::styled(t(label_key), text.add_modifier(Modifier::BOLD)),
        Span::styled(value, text),
    ])
}

/// Renders `lines` in the "Details" pane.
pub(super) fn render_details(f: &mut Frame, pane: &Pane, lines: Vec<Line<'static>>) {
    let detail_para = Paragraph::new(lines)
        .block(pane_block(pane, t("plugin_details")))
        .wrap(Wrap { trim: false });
    f.render_widget(detail_para, pane.detail_area);
}
