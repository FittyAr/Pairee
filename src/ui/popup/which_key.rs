//! Render the which-key overlay (live keymap chords).

use crate::app::actions::which_key::filter_items;
use crate::app::context::AppContext;
use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::keybindings::Action;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    theme: &Theme,
    area: Rect,
    context: &AppContext,
) -> bool {
    let PopupType::WhichKey {
        query,
        cursor_idx,
        items,
    } = popup
    else {
        return false;
    };

    let visible = filter_items(query, items);
    let chord = context
        .resolver
        .key_for_action(Action::WhichKey)
        .unwrap_or("Ctrl+Shift+k");

    let width = area.width.min(72);
    let height = area.height.min(18);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 3;
    let rect = Rect::new(x, y, width, height);

    f.render_widget(Clear, rect);
    let title = format!(" {} ({chord}) ", t("which_key_title"));
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .style(
            Style::default()
                .fg(parse_color(&theme.popup_fg))
                .bg(parse_color(&theme.popup_bg)),
        );
    let inner = block.inner(rect);
    f.render_widget(block, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let prompt = Paragraph::new(Line::from(vec![
        Span::styled("> ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(query.as_str()),
    ]))
    .style(
        Style::default()
            .fg(parse_color(&theme.popup_fg))
            .bg(parse_color(&theme.popup_bg)),
    );
    f.render_widget(prompt, chunks[0]);

    let cursor = if visible.is_empty() {
        0
    } else {
        (*cursor_idx).min(visible.len() - 1)
    };
    let list_items: Vec<ListItem> = visible
        .iter()
        .enumerate()
        .map(|(i, (bind, label, _))| {
            let style = if i == cursor {
                Style::default()
                    .fg(parse_color(&theme.selection_fg))
                    .bg(parse_color(&theme.selection_bg))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(parse_color(&theme.popup_fg))
                    .bg(parse_color(&theme.popup_bg))
            };
            ListItem::new(format!("{bind:<16} {label}")).style(style)
        })
        .collect();
    f.render_widget(List::new(list_items), chunks[1]);

    f.render_widget(
        Paragraph::new(t("which_key_hint")).style(
            Style::default()
                .fg(parse_color(&theme.popup_fg))
                .bg(parse_color(&theme.popup_bg)),
        ),
        chunks[2],
    );
    true
}
