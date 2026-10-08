//! One-line bar of folder tabs above a panel, shown when the side has more
//! than one tab (or always, with `always_show_tab_bar`).

use crate::app::state::tabs::{PanelTabs, Tab, TabId};
use crate::config::theme::Theme;
use crate::ui::text_width::{display_width, truncate_to_width};
use crate::ui::theme_apply::parse_color;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::cell::RefCell;

/// Narrowest cell a tab is squeezed to: padding plus two columns.
const MIN_CELL: usize = 4;
/// Spaces around a title inside its cell.
const PADDING: usize = 2;

/// Tab cells painted last frame, for mouse clicks.
#[derive(Debug, Default)]
pub struct TabBarHits {
    cells: RefCell<Vec<(Rect, TabId)>>,
}

impl TabBarHits {
    pub fn clear(&self) {
        self.cells.borrow_mut().clear();
    }

    fn register(&self, area: Rect, id: TabId) {
        self.cells.borrow_mut().push((area, id));
    }

    /// The tab painted at column `x`, row `y`.
    pub fn tab_at(&self, x: u16, y: u16) -> Option<TabId> {
        self.cells
            .borrow()
            .iter()
            .find(|(area, _)| area.contains(ratatui::layout::Position { x, y }))
            .map(|(_, id)| *id)
    }
}

/// Whether a side shows its tab bar.
pub fn is_visible(tabs: &PanelTabs, always: bool) -> bool {
    always || tabs.count() > 1
}

/// Text of a tab on the bar: lock mark, `Alt+N` number and title.
fn label(index: usize, tab: &Tab) -> String {
    let lock = if tab.lock.is_some() { "*" } else { "" };
    match index {
        0..=8 => format!("{lock}{}:{}", index + 1, tab.title()),
        _ => format!("{lock}{}", tab.title()),
    }
}

/// A tab's place on the bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub index: usize,
    pub x: usize,
    pub width: usize,
}

/// Lays tabs out on `total` columns: wide titles shrink first (down to
/// [`MIN_CELL`]); when even that does not fit, a window of tabs around the
/// active one is shown.
pub fn layout(natural: &[usize], active: usize, total: usize) -> Vec<Cell> {
    let floor: Vec<usize> = natural.iter().map(|&w| w.min(MIN_CELL)).collect();
    let mut widths = natural.to_vec();
    while widths.iter().sum::<usize>() > total {
        let widest = (0..widths.len())
            .filter(|&i| widths[i] > floor[i])
            .max_by_key(|&i| widths[i]);
        match widest {
            Some(i) => widths[i] -= 1,
            None => break,
        }
    }
    let (start, end) = window(&widths, active, total);
    let mut x = 0;
    (start..end)
        .map(|index| {
            let width = widths[index].min(total - x);
            let cell = Cell { index, x, width };
            x += width;
            cell
        })
        .collect()
}

/// Range of tabs that fits on `total` columns and contains `active`.
fn window(widths: &[usize], active: usize, total: usize) -> (usize, usize) {
    let (mut start, mut end) = (active, active + 1);
    let mut used = widths[active];
    loop {
        let grew_right = end < widths.len() && used + widths[end] <= total;
        if grew_right {
            used += widths[end];
            end += 1;
        }
        let grew_left = start > 0 && used + widths[start - 1] <= total;
        if grew_left {
            start -= 1;
            used += widths[start];
        }
        if !grew_right && !grew_left {
            return (start, end);
        }
    }
}

/// Paints the bar of `tabs` on `area` and records the cells for clicks.
pub fn render(
    f: &mut Frame,
    area: Rect,
    tabs: &PanelTabs,
    side_focused: bool,
    theme: &Theme,
    hits: &TabBarHits,
) {
    let labels: Vec<String> = tabs
        .tabs()
        .iter()
        .enumerate()
        .map(|(i, tab)| label(i, tab))
        .collect();
    let natural: Vec<usize> = labels.iter().map(|l| display_width(l) + PADDING).collect();
    let base = Style::default()
        .fg(parse_color(&theme.panel_fg))
        .bg(parse_color(&theme.panel_bg));
    let mut current = Style::default()
        .fg(parse_color(&theme.selection_fg))
        .bg(parse_color(&theme.selection_bg));
    if side_focused {
        current = current.add_modifier(Modifier::BOLD);
    }
    let mut spans = Vec::new();
    for cell in layout(&natural, tabs.active_index(), usize::from(area.width)) {
        let text = truncate_to_width(&labels[cell.index], cell.width.saturating_sub(PADDING));
        let pad = cell.width.saturating_sub(display_width(&text) + 1);
        let style = if cell.index == tabs.active_index() {
            current
        } else {
            base
        };
        spans.push(Span::styled(format!(" {text}{}", " ".repeat(pad)), style));
        let cell_area = Rect::new(area.x + cell.x as u16, area.y, cell.width as u16, 1);
        hits.register(cell_area, tabs.tabs()[cell.index].id);
    }
    f.render_widget(Paragraph::new(Line::from(spans)).style(base), area);
}
