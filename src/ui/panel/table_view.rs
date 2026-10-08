//! Column-based panel views (Medium, Full, Wide, Detailed, Descriptions,
//! File owners, File links) described as tables of [`Column`]s and drawn by
//! one renderer.

use super::helpers::{entry_size_text, format_date, visible_range, visible_slice};
use super::list_ctx::ListCtx;
use crate::app::state::{PanelState, PanelViewMode};
use crate::config::localization::t;
use crate::fs::FileEntry;
use crate::fs::attrs::format_unix_mode;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Cell, Row, Table},
};
use std::path::Path;

/// Text of one cell for an entry.
type CellFn = fn(&PanelState, &FileEntry) -> String;

/// A non-name column: header key, width share and cell text.
pub(crate) struct Column {
    header: &'static str,
    percent: u16,
    cell: CellFn,
}

/// How the name column is drawn.
pub(crate) enum NameCell {
    /// Whole name.
    Full,
    /// Truncated to its column's share of the inner width (at least 8).
    Share,
    /// Truncated to the whole panel width (single-column view).
    Wide,
}

impl NameCell {
    fn width(&self, area_width: u16, percent: u16) -> Option<usize> {
        let inner = area_width.saturating_sub(2) as usize;
        match self {
            Self::Full => None,
            Self::Share => Some((inner * percent as usize / 100).max(8)),
            Self::Wide => Some((area_width.saturating_sub(4) as usize).max(4)),
        }
    }
}

pub(crate) struct TableView {
    name_percent: u16,
    name: NameCell,
    columns: &'static [Column],
    /// Whether the view shows column titles (when enabled in the settings).
    headers: bool,
}

/// Size column: file or computed folder size, `dir` for unmeasured folders.
fn size_or(panel: &PanelState, entry: &FileEntry, dir: &str) -> String {
    entry_size_text(panel, entry).unwrap_or_else(|| dir.to_string())
}

const SIZE: Column = Column {
    header: "col_size",
    percent: 25,
    cell: |p, e| size_or(p, e, ""),
};

pub(crate) const MEDIUM: TableView = TableView {
    name_percent: 60,
    name: NameCell::Share,
    columns: &[
        Column {
            header: "col_ext",
            percent: 15,
            cell: |_, e| {
                if e.is_dir {
                    "<DIR>".to_string()
                } else {
                    Path::new(&e.name)
                        .extension()
                        .map(|x| x.to_string_lossy().to_uppercase())
                        .unwrap_or_default()
                }
            },
        },
        SIZE,
    ],
    headers: true,
};

pub(crate) const FULL: TableView = TableView {
    name_percent: 55,
    name: NameCell::Full,
    columns: &[
        Column {
            header: "col_size",
            percent: 15,
            cell: |p, e| size_or(p, e, "  <DIR>  "),
        },
        Column {
            header: "col_date_modified",
            percent: 30,
            cell: |_, e| format_date(e.modified),
        },
    ],
    headers: true,
};

pub(crate) const WIDE: TableView = TableView {
    name_percent: 100,
    name: NameCell::Wide,
    columns: &[],
    headers: false,
};

pub(crate) const DETAILED: TableView = TableView {
    name_percent: 40,
    name: NameCell::Share,
    columns: &[
        Column {
            header: "col_perms",
            percent: 15,
            cell: |p, e| {
                p.entry_attrs(e)
                    .map(|a| format_unix_mode(a.mode))
                    .unwrap_or_else(|| "?????????".to_string())
            },
        },
        Column {
            header: "col_owner",
            percent: 20,
            cell: owner,
        },
        Column {
            header: "col_size",
            percent: 25,
            cell: |p, e| size_or(p, e, "<DIR>"),
        },
    ],
    headers: true,
};

pub(crate) const DESCRIPTIONS: TableView = TableView {
    name_percent: 40,
    name: NameCell::Full,
    columns: &[Column {
        header: "col_description",
        percent: 60,
        cell: |p, e| {
            crate::fs::descriptions::read_description(&p.current_path, &e.name).unwrap_or_default()
        },
    }],
    headers: true,
};

pub(crate) const FILE_OWNERS: TableView = TableView {
    name_percent: 60,
    name: NameCell::Full,
    columns: &[Column {
        header: "col_owner",
        percent: 40,
        cell: owner,
    }],
    headers: true,
};

pub(crate) const FILE_LINKS: TableView = TableView {
    name_percent: 80,
    name: NameCell::Full,
    columns: &[Column {
        header: "col_links",
        percent: 20,
        cell: |p, e| {
            p.entry_attrs(e)
                .map(|a| a.nlinks.to_string())
                .unwrap_or_else(|| "?".to_string())
        },
    }],
    headers: true,
};

/// The table description of `mode` (`None` for the two-column Brief view).
pub(crate) fn for_mode(mode: PanelViewMode) -> Option<&'static TableView> {
    Some(match mode {
        PanelViewMode::Brief => return None,
        PanelViewMode::Medium => &MEDIUM,
        PanelViewMode::Wide => &WIDE,
        PanelViewMode::Detailed => &DETAILED,
        PanelViewMode::Descriptions => &DESCRIPTIONS,
        PanelViewMode::FileOwners => &FILE_OWNERS,
        PanelViewMode::FileLinks => &FILE_LINKS,
        PanelViewMode::Full | PanelViewMode::AltFull => &FULL,
    })
}

fn owner(panel: &PanelState, entry: &FileEntry) -> String {
    panel
        .entry_attrs(entry)
        .map(|a| a.owner.clone())
        .unwrap_or_else(|| "?".to_string())
}

/// Draws `view` for the panel in `ctx` inside `block`.
pub(crate) fn render_table(
    f: &mut Frame,
    area: Rect,
    block: Block,
    ctx: &ListCtx,
    view: &TableView,
) {
    let settings = &ctx.context.config.settings;
    let headers = view.headers && settings.show_column_titles;
    let height = area.height.saturating_sub(if headers { 3 } else { 2 }) as usize;
    let (start, end) = visible_range(ctx.panel, height);
    let name_width = view.name.width(area.width, view.name_percent);

    let rows: Vec<Row> = visible_slice(ctx.panel, start, end)
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let mut cells = vec![Cell::from(ctx.name(entry, name_width))];
            cells.extend(
                view.columns
                    .iter()
                    .map(|c| Cell::from((c.cell)(ctx.panel, entry))),
            );
            Row::new(cells).style(ctx.row_style(start + rel, entry))
        })
        .collect();

    let widths = std::iter::once(view.name_percent)
        .chain(view.columns.iter().map(|c| c.percent))
        .map(Constraint::Percentage);
    let mut table = Table::new(rows, widths).block(block);
    if headers {
        let titles = std::iter::once("col_name")
            .chain(view.columns.iter().map(|c| c.header))
            .map(t);
        table = table.header(
            Row::new(titles.collect::<Vec<_>>()).style(
                Style::default()
                    .fg(parse_color(&ctx.context.config.theme.header_fg))
                    .add_modifier(Modifier::BOLD),
            ),
        );
    }
    f.render_widget(table, area);
}
