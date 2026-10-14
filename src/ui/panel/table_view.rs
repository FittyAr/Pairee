//! Column-based panel views (Medium, Full, Wide, Detailed, Descriptions,
//! File owners, File links) described as tables of [`Column`]s and drawn by
//! one renderer.

use super::helpers::{entry_size_text, format_date, visible_slice};
use super::list_ctx::ListCtx;
use crate::app::state::{PanelState, PanelViewMode, SortField};
use crate::config::localization::t;
use crate::fs::FileEntry;
use crate::fs::attrs::format_unix_mode;
use crate::fs::descriptions::{find_description, read_descriptions_file};
use crate::ui::hit::{self, HitTarget};
use crate::ui::scrollbar::ScrollView;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Cell, Row, Table},
};
use std::cell::OnceCell;
use std::path::Path;

/// Text of one cell for an entry.
type CellFn = fn(&Cells, &FileEntry) -> String;

/// What the cells of one frame read: the panel and the folder's
/// `descript.ion`, loaded on first use (once per frame, not once per row).
pub(crate) struct Cells<'a> {
    panel: &'a PanelState,
    descriptions: OnceCell<Option<String>>,
}

impl<'a> Cells<'a> {
    fn new(panel: &'a PanelState) -> Self {
        Self {
            panel,
            descriptions: OnceCell::new(),
        }
    }

    /// Description of `entry`, or empty.
    fn description(&self, entry: &FileEntry) -> String {
        self.descriptions
            .get_or_init(|| read_descriptions_file(&self.panel.current_path))
            .as_deref()
            .and_then(|content| find_description(content, &entry.name))
            .unwrap_or_default()
            .to_string()
    }
}

/// A non-name column: header key, width share, cell text and the sort its
/// title selects when clicked.
pub(crate) struct Column {
    header: &'static str,
    percent: u16,
    cell: CellFn,
    sort: Option<SortField>,
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
    cell: |c, e| size_or(c.panel, e, ""),
    sort: Some(SortField::Size),
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
            sort: Some(SortField::Extension),
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
            cell: |c, e| size_or(c.panel, e, "  <DIR>  "),
            sort: Some(SortField::Size),
        },
        Column {
            header: "col_date_modified",
            percent: 30,
            cell: |_, e| format_date(e.modified),
            sort: Some(SortField::Date),
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
            cell: |c, e| {
                c.panel
                    .entry_attrs(e)
                    .map(|a| format_unix_mode(a.mode))
                    .unwrap_or_else(|| "?????????".to_string())
            },
            sort: None,
        },
        Column {
            header: "col_owner",
            percent: 20,
            cell: owner,
            sort: None,
        },
        Column {
            header: "col_size",
            percent: 25,
            cell: |c, e| size_or(c.panel, e, "<DIR>"),
            sort: Some(SortField::Size),
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
        cell: |c, e| c.description(e),
        sort: None,
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
        sort: None,
    }],
    headers: true,
};

pub(crate) const FILE_LINKS: TableView = TableView {
    name_percent: 80,
    name: NameCell::Full,
    columns: &[Column {
        header: "col_links",
        percent: 20,
        cell: |c, e| {
            c.panel
                .entry_attrs(e)
                .map(|a| a.nlinks.to_string())
                .unwrap_or_else(|| "?".to_string())
        },
        sort: None,
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

fn owner(cells: &Cells, entry: &FileEntry) -> String {
    cells
        .panel
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
) -> ScrollView {
    let settings = &ctx.context.config.settings;
    let headers = view.headers && settings.show_column_titles;
    let height = area.height.saturating_sub(if headers { 3 } else { 2 }) as usize;
    let (start, end) = ctx.range(height);
    let name_width = view.name.width(area.width, view.name_percent);
    let source = Cells::new(ctx.panel);

    let rows: Vec<Row> = visible_slice(ctx.panel, start, end)
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let mut cells = vec![Cell::from(ctx.name(entry, name_width))];
            cells.extend(
                view.columns
                    .iter()
                    .map(|c| Cell::from((c.cell)(&source, entry))),
            );
            Row::new(cells).style(ctx.row_style(start + rel, entry))
        })
        .collect();

    let widths: Vec<Constraint> = std::iter::once(view.name_percent)
        .chain(view.columns.iter().map(|c| c.percent))
        .map(Constraint::Percentage)
        .collect();
    let inner = block.inner(area);
    let header_rows = u16::from(headers);
    if headers {
        register_headers(ctx, view, inner, &widths);
    }
    ctx.register_rows(
        Rect {
            y: inner.y + header_rows,
            height: inner.height.saturating_sub(header_rows),
            ..inner
        },
        start,
    );
    let mut table = Table::new(rows, widths).block(block);
    if headers {
        let titles = std::iter::once("col_name")
            .chain(view.columns.iter().map(|c| c.header))
            .map(t);
        table = table.header(
            Row::new(titles.collect::<Vec<_>>()).style(
                Style::default()
                    .fg(ctx.context.theme().header_fg)
                    .add_modifier(Modifier::BOLD),
            ),
        );
    }
    f.render_widget(table, area);
    ScrollView {
        content_len: ctx.panel.entries.len(),
        viewport_len: height,
        offset: start,
    }
}

/// Records the column titles on the first inner row; the columns are laid
/// out as [`Table`] lays them (one blank column between cells).
fn register_headers(ctx: &ListCtx, view: &TableView, inner: Rect, widths: &[Constraint]) {
    let row = Rect { height: 1, ..inner };
    let cells = Layout::horizontal(widths.iter().copied())
        .spacing(1)
        .split(row);
    let sorts = std::iter::once(Some(SortField::Name)).chain(view.columns.iter().map(|c| c.sort));
    for (cell, sort) in cells.iter().zip(sorts) {
        hit::register(
            *cell,
            HitTarget::PanelHeader {
                side: ctx.side,
                sort,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptions_are_read_once_per_frame() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(crate::fs::descriptions::DESCRIPTIONS_FILE);
        std::fs::write(&file, "a.txt First\n\"b c.txt\" Second\n").unwrap();
        let panel = PanelState::new(dir.path().to_path_buf());
        let entry = |name: &str| FileEntry {
            name: name.into(),
            path: dir.path().join(name),
            size: 0,
            is_dir: false,
            is_symlink: false,
            modified: None,
        };
        let cells = Cells::new(&panel);
        assert_eq!(cells.description(&entry("A.TXT")), "First");
        // Later rows of the same frame reuse the text read for the first.
        std::fs::remove_file(&file).unwrap();
        assert_eq!(cells.description(&entry("b c.txt")), "Second");
        assert_eq!(cells.description(&entry("none")), "");
        assert_eq!(Cells::new(&panel).description(&entry("a.txt")), "");
    }
}
