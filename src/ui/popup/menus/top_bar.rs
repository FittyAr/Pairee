//! Rendering of the top navigation dropdown menu and cascading submenus.

use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::config::theme::Theme;
use crate::ui::menu::MenuItemData;
use crate::ui::popup::kit;
use crate::ui::theme_apply::parse_color;
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
/// Column of each top menu's drop-down.
const DROPDOWN_X: [u16; 5] = [2, 10, 19, 31, 42];

pub fn render_top_dropdown(
    f: &mut Frame,
    theme: &Theme,
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
    let x = DROPDOWN_X.get(focus.menu).copied().unwrap_or(DROPDOWN_X[0]);
    let items = items_of(focus.menu);
    let rect = Rect::new(x, 1, DROPDOWN_WIDTH, (items.len() + 2) as u16).intersection(size);
    // While a submenu is open its parent row is highlighted differently.
    let parent = Style::default()
        .bg(parse_color("Blue"))
        .fg(parse_color("White"))
        .add_modifier(Modifier::BOLD);
    let highlight = match focus.submenu {
        Some(_) => (item, parent),
        None => (item, selected(theme)),
    };
    render_box(f, rect, &items, highlight, theme);

    if let (Some(sub), Some(sub_item)) = (focus.submenu, focus.sub_item) {
        let sub_items = items_of(sub);
        let mut sub_x = x + DROPDOWN_WIDTH;
        if sub_x + DROPDOWN_WIDTH > size.width {
            sub_x = x.saturating_sub(DROPDOWN_WIDTH);
        }
        let sub_y = item as u16 + 2;
        let sub_rect = Rect::new(sub_x, sub_y, DROPDOWN_WIDTH, (sub_items.len() + 2) as u16)
            .intersection(size);
        render_box(f, sub_rect, &sub_items, (sub_item, selected(theme)), theme);
    }
}

fn selected(theme: &Theme) -> Style {
    kit::selection(theme).add_modifier(Modifier::BOLD)
}

/// Draws `items` in a bordered box, row `highlight.0` in style `highlight.1`.
fn render_box(
    f: &mut Frame,
    rect: Rect,
    items: &[MenuItemData],
    (highlight_row, highlight): (usize, Style),
    theme: &Theme,
) {
    f.render_widget(Clear, rect);
    let normal = kit::popup_fg(theme);
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
        .border_style(Style::default().fg(parse_color(&theme.popup_border)))
        .style(Style::default().bg(parse_color(&theme.popup_bg)));
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
    let label_len = crate::ui::hotkey::parse_hotkey(&item.label)
        .clean_text
        .chars()
        .count();
    spans.push(Span::styled(
        " ".repeat(25usize.saturating_sub(label_len)),
        base,
    ));
    spans.push(Span::styled(format!("{:<7}", item.shortcut), base));
    Line::from(spans)
}
