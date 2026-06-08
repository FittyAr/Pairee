use crate::app::context::AppContext;
use crate::app::state::{PanelState, PanelViewMode};
use crate::fs::attrs::format_unix_mode;
use crate::fs::descriptions::read_description;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

/// Entry point: dispatches to the correct view mode renderer.
pub fn render_panel(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
) {
    let theme = &context.config.theme;

    let border_color = if is_active {
        parse_color(&theme.panel_border)
    } else {
        parse_color("DarkGray")
    };

    let mode_label = match panel.view_mode {
        PanelViewMode::Brief => "Brief",
        PanelViewMode::Medium => "Medium",
        PanelViewMode::Full => "Full",
        PanelViewMode::Wide => "Wide",
        PanelViewMode::Detailed => "Detailed",
        PanelViewMode::Descriptions => "Desc",
        PanelViewMode::FileOwners => "Owners",
        PanelViewMode::FileLinks => "Links",
        PanelViewMode::AltFull => "Alt",
    };

    let title = format!(
        " {} [{}] ",
        panel.current_path.to_string_lossy(),
        mode_label
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    match panel.view_mode {
        PanelViewMode::Brief => render_brief(f, area, panel, is_active, context, block),
        PanelViewMode::Medium => render_medium(f, area, panel, is_active, context, block),
        PanelViewMode::Wide => render_wide(f, area, panel, is_active, context, block),
        PanelViewMode::Detailed => render_detailed(f, area, panel, is_active, context, block),
        PanelViewMode::Descriptions => {
            render_descriptions(f, area, panel, is_active, context, block)
        }
        PanelViewMode::FileOwners => render_file_owners(f, area, panel, is_active, context, block),
        PanelViewMode::FileLinks => render_file_links(f, area, panel, is_active, context, block),
        // Full + AltFull use the full 3-column layout
        PanelViewMode::Full | PanelViewMode::AltFull => {
            render_full(f, area, panel, is_active, context, block)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

fn visible_range(panel: &PanelState, height: usize) -> (usize, usize) {
    let start = if panel.cursor_index > height / 2 {
        panel.cursor_index.saturating_sub(height / 2)
    } else {
        0
    };
    let start = if start + height > panel.entries.len() {
        panel.entries.len().saturating_sub(height)
    } else {
        start
    };
    (start, start + height.min(panel.entries.len().saturating_sub(start)))
}

fn build_row_style(
    is_cursor: bool,
    is_selected: bool,
    is_active: bool,
    theme: &crate::config::theme::Theme,
) -> Style {
    let mut style = Style::default().fg(parse_color(&theme.panel_fg));
    if is_selected {
        style = style.fg(parse_color(&theme.marked_fg));
    }
    if is_cursor && is_active {
        style = style
            .bg(parse_color(&theme.selection_bg))
            .fg(parse_color(&theme.selection_fg))
            .add_modifier(Modifier::BOLD);
    } else if is_cursor && !is_active {
        style = style.bg(parse_color("DarkGray"));
    }
    style
}

fn entry_display_name(name: &str, is_dir: bool) -> String {
    if is_dir && name != ".." {
        format!("/{}", name)
    } else {
        name.to_string()
    }
}

fn format_file_size(size: u64) -> String {
    if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn format_date(time: Option<std::time::SystemTime>) -> String {
    match time {
        Some(t) => {
            let dt: chrono::DateTime<chrono::Local> = t.into();
            dt.format("%d/%m/%Y %H:%M").to_string()
        }
        None => String::new(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Full view: Name │ Size │ Date
// ─────────────────────────────────────────────────────────────────────────────

fn render_full(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let height = area.height.saturating_sub(3) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
            );
            Row::new(vec![
                Cell::from(entry_display_name(&entry.name, entry.is_dir)),
                Cell::from(if entry.is_dir {
                    "  <DIR>  ".to_string()
                } else {
                    format_file_size(entry.size)
                }),
                Cell::from(format_date(entry.modified)),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "Size", "Date Modified"]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(15),
            Constraint::Percentage(30),
        ],
    )
    .header(header)
    .block(block);
    f.render_widget(table, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Medium view: Name │ Ext │ Size
// ─────────────────────────────────────────────────────────────────────────────

fn render_medium(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let height = area.height.saturating_sub(3) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
            );
            let ext = if entry.is_dir {
                "<DIR>".to_string()
            } else {
                std::path::Path::new(&entry.name)
                    .extension()
                    .map(|e| e.to_string_lossy().to_uppercase())
                    .unwrap_or_default()
            };
            Row::new(vec![
                Cell::from(entry_display_name(&entry.name, entry.is_dir)),
                Cell::from(ext),
                Cell::from(if entry.is_dir {
                    String::new()
                } else {
                    format_file_size(entry.size)
                }),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "Ext", "Size"]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(60),
            Constraint::Percentage(15),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(block);
    f.render_widget(table, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Wide view: single wide column of names
// ─────────────────────────────────────────────────────────────────────────────

fn render_wide(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let height = area.height.saturating_sub(2) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
            );
            Row::new(vec![Cell::from(entry_display_name(&entry.name, entry.is_dir))]).style(style)
        })
        .collect();

    let table = Table::new(rows, [Constraint::Percentage(100)]).block(block);
    f.render_widget(table, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Detailed view: Name │ Perms │ Owner │ Size
// ─────────────────────────────────────────────────────────────────────────────

fn render_detailed(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let height = area.height.saturating_sub(3) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
            );
            let (perm_str, owner) = if let Ok(attrs) =
                crate::fs::attrs::read_attrs(&entry.path)
            {
                (format_unix_mode(attrs.mode), attrs.owner)
            } else {
                ("?????????".to_string(), "?".to_string())
            };
            Row::new(vec![
                Cell::from(entry_display_name(&entry.name, entry.is_dir)),
                Cell::from(perm_str),
                Cell::from(owner),
                Cell::from(if entry.is_dir {
                    "<DIR>".to_string()
                } else {
                    format_file_size(entry.size)
                }),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "Perms", "Owner", "Size"]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
        ],
    )
    .header(header)
    .block(block);
    f.render_widget(table, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Descriptions view: Name │ Description (from descript.ion)
// ─────────────────────────────────────────────────────────────────────────────

fn render_descriptions(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let height = area.height.saturating_sub(3) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
            );
            let desc =
                read_description(&panel.current_path, &entry.name).unwrap_or_default();
            Row::new(vec![
                Cell::from(entry_display_name(&entry.name, entry.is_dir)),
                Cell::from(desc),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "Description"]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [Constraint::Percentage(40), Constraint::Percentage(60)],
    )
    .header(header)
    .block(block);
    f.render_widget(table, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// File Owners view: Name │ Owner │ Group (approximated as owner)
// ─────────────────────────────────────────────────────────────────────────────

fn render_file_owners(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let height = area.height.saturating_sub(3) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
            );
            let owner = crate::fs::attrs::read_attrs(&entry.path)
                .map(|a| a.owner)
                .unwrap_or_else(|_| "?".to_string());
            Row::new(vec![
                Cell::from(entry_display_name(&entry.name, entry.is_dir)),
                Cell::from(owner),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "Owner"]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [Constraint::Percentage(60), Constraint::Percentage(40)],
    )
    .header(header)
    .block(block);
    f.render_widget(table, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// File Links view: Name │ #Links
// ─────────────────────────────────────────────────────────────────────────────

fn render_file_links(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let height = area.height.saturating_sub(3) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
            );
            let nlinks = crate::fs::attrs::read_attrs(&entry.path)
                .map(|a| a.nlinks.to_string())
                .unwrap_or_else(|_| "?".to_string());
            Row::new(vec![
                Cell::from(entry_display_name(&entry.name, entry.is_dir)),
                Cell::from(nlinks),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["Name", "#Links"]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [Constraint::Percentage(80), Constraint::Percentage(20)],
    )
    .header(header)
    .block(block);
    f.render_widget(table, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Brief view: two-column filename grid
// ─────────────────────────────────────────────────────────────────────────────

fn render_brief(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let col_height = cols[0].height.saturating_sub(2) as usize;
    let total_visible = col_height * 2;

    let start = if panel.cursor_index > total_visible / 2 {
        panel.cursor_index.saturating_sub(total_visible / 2)
    } else {
        0
    };
    let start = if start + total_visible > panel.entries.len() {
        panel.entries.len().saturating_sub(total_visible)
    } else {
        start
    };

    for col_idx in 0..2usize {
        let col_start = start + col_idx * col_height;
        let rows: Vec<Row> = panel
            .entries
            .iter()
            .enumerate()
            .skip(col_start)
            .take(col_height)
            .map(|(i, entry)| {
                let style = build_row_style(
                    i == panel.cursor_index,
                    panel.selected_paths.contains(&entry.path),
                    is_active,
                    theme,
                );
                Row::new(vec![Cell::from(entry_display_name(&entry.name, entry.is_dir))])
                    .style(style)
            })
            .collect();

        let table = Table::new(rows, [Constraint::Percentage(100)])
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(table, cols[col_idx]);
    }
}
