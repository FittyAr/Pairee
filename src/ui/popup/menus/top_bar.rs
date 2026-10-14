//! Rendering of the top navigation dropdown menu and cascading submenus.

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::ui::menu::MenuItemData;
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Which menu, item, submenu and submenu item are open / focused.
#[derive(Debug, Clone, Copy)]
pub struct MenuFocus {
    pub menu: usize,
    pub item: Option<usize>,
    pub submenu: Option<usize>,
    pub sub_item: Option<usize>,
}

/// Width of a drop-down (and of a submenu).
const DROPDOWN_WIDTH: u16 = 37;

pub fn render_top_dropdown(
    f: &mut Frame,
    theme: &ResolvedTheme,
    size: Rect,
    (state, context): (&AppState, &AppContext),
    focus: MenuFocus,
) {
    // Without an item only the top menu bar is active: no drop-down.
    let Some(item) = focus.item else {
        return;
    };
    let items_of = |menu| {
        crate::ui::menu::get_menu_items(menu, state, &context.resolver, &context.config.settings)
    };
    // Each drop-down opens under its title (the bar is the first row).
    let x = crate::ui::menu::title_cells(Rect { height: 1, ..size })
        .get(focus.menu)
        .map_or(0, |cell| cell.x);
    let items = items_of(focus.menu);
    let rect = Rect::new(x, 1, DROPDOWN_WIDTH, (items.len() + 2) as u16).intersection(size);
    // While a submenu is open its parent row is highlighted differently.
    let parent = Style::default()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let highlight = match focus.submenu {
        Some(_) => (item, parent),
        None => (item, selected(theme)),
    };
    render_box(f, rect, (focus.menu, &items), highlight, theme);

    if let (Some(sub), Some(sub_item)) = (focus.submenu, focus.sub_item) {
        let sub_items = items_of(sub);
        let mut sub_x = x + DROPDOWN_WIDTH;
        if sub_x + DROPDOWN_WIDTH > size.width {
            sub_x = x.saturating_sub(DROPDOWN_WIDTH);
        }
        let sub_y = item as u16 + 2;
        let sub_rect = Rect::new(sub_x, sub_y, DROPDOWN_WIDTH, (sub_items.len() + 2) as u16)
            .intersection(size);
        render_box(
            f,
            sub_rect,
            (sub, &sub_items),
            (sub_item, selected(theme)),
            theme,
        );
    }
}

fn selected(theme: &ResolvedTheme) -> Style {
    theme.selection().add_modifier(Modifier::BOLD)
}

/// Draws the `items` of menu `menu` in a bordered box, row `highlight.0` in
/// style `highlight.1`, and records the rows for the mouse.
fn render_box(
    f: &mut Frame,
    rect: Rect,
    (menu, items): (usize, &[MenuItemData]),
    (highlight_row, highlight): (usize, Style),
    theme: &ResolvedTheme,
) {
    crate::ui::popup::popup_body(rect);
    let rows = rect.inner(ratatui::layout::Margin::new(1, 1));
    for (row, item) in items.iter().enumerate() {
        let y = rows.y + row as u16;
        if !item.is_separator && y < rows.bottom() {
            crate::ui::hit::register(
                Rect::new(rows.x, y, rows.width, 1),
                crate::ui::hit::HitTarget::MenuItem { menu, row },
            );
        }
    }
    f.render_widget(Clear, rect);
    let normal = theme.popup_fg();
    let lines: Vec<Line> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_cursor = i == highlight_row;
            item_line(item, if is_cursor { highlight } else { normal }, is_cursor)
        })
        .collect();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.popup_border))
        .style(Style::default().bg(theme.popup_bg));
    f.render_widget(Paragraph::new(lines).block(block), rect);
}

/// One drop-down row: active marker, label with its hotkey, shortcut.
fn item_line(item: &MenuItemData, base: Style, is_cursor: bool) -> Line<'static> {
    if item.is_separator {
        return Line::from(Span::styled(" ───────────────────────────────── ", base));
    }
    let hotkey = if is_cursor {
        base.fg(Color::Yellow)
    } else {
        base.fg(Color::Yellow).add_modifier(Modifier::BOLD)
    };
    let marker = if item.active { "•" } else { " " };
    let mut spans = vec![Span::styled(format!(" {} ", marker), base)];
    spans.extend(crate::ui::hotkey::render_hotkey_spans(
        &item.label,
        base,
        hotkey,
    ));
    let label_len = crate::ui::text_width::display_width(
        &crate::ui::hotkey::parse_hotkey(&item.label).clean_text,
    );
    spans.push(Span::styled(
        " ".repeat(25usize.saturating_sub(label_len)),
        base,
    ));
    spans.push(Span::styled(
        crate::ui::text_width::pad_to_width(&item.shortcut, 7),
        base,
    ));
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_column_lines_up_with_wide_labels() {
        let width = |label: &str| {
            let item = MenuItemData::new(label.into(), "Ctrl+X", false);
            item_line(&item, Style::default(), false).width()
        };
        assert_eq!(width("&Copiar"), width("&日本語メニュー"));
    }
}
