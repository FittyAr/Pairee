use crate::config::localization::t;
use crate::app::context::AppContext;
use crate::app::state::{PanelState, PanelViewMode, SortField};
use crate::fs::attrs::format_unix_mode;
use crate::fs::descriptions::read_description;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table,
    },
};

/// Entry point: dispatches to the correct view mode renderer.
/// Also renders optional footer lines (status, total info, free space) and scrollbar.
pub fn render_panel(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
) {
    let theme = &context.config.theme;
    let settings = &context.config.settings;

    let border_color = if is_active {
        parse_color(&theme.panel_border)
    } else {
        parse_color("DarkGray")
    };

    // ── Build panel title with optional sort mode letter ──────────────────────
    let mode_label = match panel.view_mode {
        PanelViewMode::Brief => t("panel_mode_brief"),
        PanelViewMode::Medium => t("panel_mode_medium"),
        PanelViewMode::Full => t("panel_mode_full"),
        PanelViewMode::Wide => t("panel_mode_wide"),
        PanelViewMode::Detailed => t("panel_mode_detailed"),
        PanelViewMode::Descriptions => t("panel_mode_desc"),
        PanelViewMode::FileOwners => t("panel_mode_owners"),
        PanelViewMode::FileLinks => t("panel_mode_links"),
        PanelViewMode::AltFull => t("panel_mode_alt"),
    };

    let sort_letter = if settings.show_sort_mode_letter {
        let letter = match panel.sort_field {
            SortField::Name => "N",
            SortField::Extension => "X",
            SortField::Size => "S",
            SortField::Date => "D",
            SortField::Unsorted => "U",
        };
        let rev = if panel.sort_reverse { "▼" } else { "▲" };
        format!("|{}{}", letter, rev)
    } else {
        String::new()
    };

    let title = format!(
        " {} [{}{}] ",
        panel.current_path.to_string_lossy(),
        mode_label,
        sort_letter,
    );

    // ── Count optional footer rows ────────────────────────────────────────────
    let show_status = settings.show_status_line;
    let show_total = settings.show_files_total_information;
    let show_free = settings.show_free_size;
    let show_scrollbar = settings.show_scrollbar;
    let highlight_files = settings.highlight_files;

    let footer_height = u16::from(show_status) + u16::from(show_total) + u16::from(show_free);

    // ── Split area: [block_with_list] + [footer lines] ────────────────────────
    let constraints: Vec<Constraint> = if footer_height > 0 {
        vec![Constraint::Min(3), Constraint::Length(footer_height)]
    } else {
        vec![Constraint::Percentage(100)]
    };

    let v_split = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let list_area = v_split[0];
    let footer_area = if footer_height > 0 {
        Some(v_split[1])
    } else {
        None
    };

    // ── Build the panel block ─────────────────────────────────────────────────
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    // ── Dispatch to view-specific renderer (list area only) ───────────────────
    match panel.view_mode {
        PanelViewMode::Brief => render_brief(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Medium => render_medium(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Wide => render_wide(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Detailed => render_detailed(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Descriptions => render_descriptions(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::FileOwners => render_file_owners(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::FileLinks => render_file_links(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
        PanelViewMode::Full | PanelViewMode::AltFull => render_full(
            f,
            list_area,
            panel,
            is_active,
            context,
            block,
            highlight_files,
        ),
    }

    // ── Optional scrollbar ────────────────────────────────────────────────────
    if show_scrollbar && !panel.entries.is_empty() {
        let inner_height = list_area.height.saturating_sub(2) as usize;
        let total = panel.entries.len();
        let mut scrollbar_state = ScrollbarState::new(total.saturating_sub(inner_height))
            .position(panel.cursor_index.min(total.saturating_sub(inner_height)));
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
        let scrollbar_area = Rect {
            x: list_area.x + list_area.width.saturating_sub(1),
            y: list_area.y + 1,
            width: 1,
            height: list_area.height.saturating_sub(2),
        };
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }

    // ── Optional footer lines ─────────────────────────────────────────────────
    if let Some(footer_area) = footer_area {
        let total_files = panel.entries.iter().filter(|e| !e.is_dir).count();
        let total_dirs = panel
            .entries
            .iter()
            .filter(|e| e.is_dir && e.name != "..")
            .count();
        let total_size: u64 = panel
            .entries
            .iter()
            .filter(|e| !e.is_dir)
            .map(|e| e.size)
            .sum();
        let tagged = panel.selected_paths.len();

        let fg = Style::default()
            .fg(parse_color(&theme.panel_fg))
            .bg(parse_color(&theme.panel_bg));

        let mut footer_lines: Vec<Line> = Vec::new();

        if show_status {
            // Status: highlighted entry name + size
            let status_text = if let Some(entry) = panel.entries.get(panel.cursor_index) {
                if entry.is_dir {
                    format!(" {} [DIR]  {} {}", entry.name, tagged, t("label_tagged"))
                } else {
                    format!(
                        " {}  {}  {} {}",
                        entry.name,
                        format_file_size(entry.size),
                        tagged,
                        t("label_tagged")
                    )
                }
            } else {
                String::new()
            };
            footer_lines.push(Line::from(Span::styled(status_text, fg)));
        }

        if show_total {
            let files_label = if total_files == 1 { t("label_file") } else { t("label_files") };
            let dirs_label = if total_dirs == 1 { t("label_dir") } else { t("label_dirs") };
            let info_text = format!(
                " {} {}  {} {}  {}",
                total_files,
                files_label,
                total_dirs,
                dirs_label,
                format_file_size(total_size),
            );
            footer_lines.push(Line::from(Span::styled(info_text, fg)));
        }

        if show_free {
            // Free space is stored at the AppState level; we show a placeholder if not available.
            // Since panel.rs doesn't have direct access to AppState free_space fields,
            // we show disk info via a quick statfs-like check here.
            let free_text = get_free_space_text(&panel.current_path);
            footer_lines.push(Line::from(Span::styled(
                format!(" {} {}", t("label_free"), free_text),
                Style::default()
                    .fg(Color::Green)
                    .bg(parse_color(&theme.panel_bg)),
            )));
        }

        if !footer_lines.is_empty() {
            let paragraph = Paragraph::new(footer_lines);
            f.render_widget(paragraph, footer_area);
        }
    }
}

/// Returns a human-readable free space string for the disk containing `path`.
fn get_free_space_text(path: &std::path::Path) -> String {
    match crate::app::sys_helpers::get_free_space(path) {
        Some(bytes) => format_file_size(bytes),
        None => "?".to_string(),
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
    (
        start,
        start + height.min(panel.entries.len().saturating_sub(start)),
    )
}

fn build_row_style(
    entry: &crate::fs::FileEntry,
    is_cursor: bool,
    is_selected: bool,
    is_active: bool,
    theme: &crate::config::theme::Theme,
    highlight_files: bool,
) -> Style {
    let base_style = Style::default().fg(parse_color(&theme.panel_fg));
    let mut style = if highlight_files {
        let rules = crate::ui::highlight::default_highlight_rules();
        crate::ui::highlight::style_for_entry(entry, &rules, base_style)
    } else {
        base_style
    };
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
    highlight_files: bool,
) {
    let theme = &context.config.theme;
    let header_offset = if context.config.settings.show_column_titles {
        3
    } else {
        2
    };
    let height = area.height.saturating_sub(header_offset) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
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

    let header = Row::new(vec![t("col_name"), t("col_size"), t("col_date_modified")]).style(
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
    );
    let table = if context.config.settings.show_column_titles {
        table.header(header)
    } else {
        table
    };
    let table = table.block(block);
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
    highlight_files: bool,
) {
    let theme = &context.config.theme;
    let header_offset = if context.config.settings.show_column_titles {
        3
    } else {
        2
    };
    let height = area.height.saturating_sub(header_offset) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
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

    let header = Row::new(vec![t("col_name"), t("col_ext"), t("col_size")]).style(
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
    );
    let table = if context.config.settings.show_column_titles {
        table.header(header)
    } else {
        table
    };
    let table = table.block(block);
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
    highlight_files: bool,
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
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
            );
            Row::new(vec![Cell::from(entry_display_name(
                &entry.name,
                entry.is_dir,
            ))])
            .style(style)
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
    highlight_files: bool,
) {
    let theme = &context.config.theme;
    let header_offset = if context.config.settings.show_column_titles {
        3
    } else {
        2
    };
    let height = area.height.saturating_sub(header_offset) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
            );
            let (perm_str, owner) = if let Ok(attrs) = crate::fs::attrs::read_attrs(&entry.path) {
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

    let header = Row::new(vec![t("col_name"), t("col_perms"), t("col_owner"), t("col_size")]).style(
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
    );
    let table = if context.config.settings.show_column_titles {
        table.header(header)
    } else {
        table
    };
    let table = table.block(block);
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
    highlight_files: bool,
) {
    let theme = &context.config.theme;
    let header_offset = if context.config.settings.show_column_titles {
        3
    } else {
        2
    };
    let height = area.height.saturating_sub(header_offset) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
            );
            let desc = read_description(&panel.current_path, &entry.name).unwrap_or_default();
            Row::new(vec![
                Cell::from(entry_display_name(&entry.name, entry.is_dir)),
                Cell::from(desc),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec![t("col_name"), t("col_description")]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [Constraint::Percentage(40), Constraint::Percentage(60)],
    );
    let table = if context.config.settings.show_column_titles {
        table.header(header)
    } else {
        table
    };
    let table = table.block(block);
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
    highlight_files: bool,
) {
    let theme = &context.config.theme;
    let header_offset = if context.config.settings.show_column_titles {
        3
    } else {
        2
    };
    let height = area.height.saturating_sub(header_offset) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
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

    let header = Row::new(vec![t("col_name"), t("col_owner")]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [Constraint::Percentage(60), Constraint::Percentage(40)],
    );
    let table = if context.config.settings.show_column_titles {
        table.header(header)
    } else {
        table
    };
    let table = table.block(block);
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
    highlight_files: bool,
) {
    let theme = &context.config.theme;
    let header_offset = if context.config.settings.show_column_titles {
        3
    } else {
        2
    };
    let height = area.height.saturating_sub(header_offset) as usize;
    let (start, end) = visible_range(panel, height);

    let rows: Vec<Row> = panel.entries[start..end]
        .iter()
        .enumerate()
        .map(|(rel, entry)| {
            let i = start + rel;
            let style = build_row_style(
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
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

    let header = Row::new(vec![t("col_name"), t("col_links")]).style(
        Style::default()
            .fg(parse_color(&theme.header_fg))
            .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(
        rows,
        [Constraint::Percentage(80), Constraint::Percentage(20)],
    );
    let table = if context.config.settings.show_column_titles {
        table.header(header)
    } else {
        table
    };
    let table = table.block(block);
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
    highlight_files: bool,
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
                    entry,
                    i == panel.cursor_index,
                    panel.selected_paths.contains(&entry.path),
                    is_active,
                    theme,
                    highlight_files,
                );
                Row::new(vec![Cell::from(entry_display_name(
                    &entry.name,
                    entry.is_dir,
                ))])
                .style(style)
            })
            .collect();

        let table = Table::new(rows, [Constraint::Percentage(100)])
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(table, cols[col_idx]);
    }
}
