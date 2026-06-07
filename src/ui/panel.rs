use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub fn render_panel(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    brief_view: bool,
) {
    let theme = &context.config.theme;

    // 1. Determine border color based on keyboard focus status
    let border_color = if is_active {
        parse_color(&theme.panel_border)
    } else {
        parse_color("DarkGray")
    };

    let title = format!(" {} ", panel.current_path.to_string_lossy());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title)
        .style(Style::default().bg(parse_color(&theme.panel_bg)));

    if brief_view {
        render_brief(f, area, panel, is_active, context, block);
    } else {
        render_full(f, area, panel, is_active, context, block);
    }
}

/// Full view: three columns — name, size, date modified.
fn render_full(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
    block: Block,
) {
    let theme = &context.config.theme;
    let mut rows = Vec::new();

    for (i, entry) in panel.entries.iter().enumerate() {
        let is_selected = panel.selected_paths.contains(&entry.path);
        let is_cursor = i == panel.cursor_index;

        let text_style = build_row_style(is_cursor, is_selected, is_active, theme);

        let name_cell = Cell::from(if entry.is_dir && entry.name != ".." {
            format!("/{}", entry.name)
        } else {
            entry.name.clone()
        });

        let size_cell = Cell::from(if entry.is_dir {
            "  <DIR>  ".to_string()
        } else {
            format_file_size(entry.size)
        });

        let date_cell = Cell::from(match entry.modified {
            Some(time) => {
                let datetime: chrono::DateTime<chrono::Local> = time.into();
                datetime.format("%d/%m/%Y %H:%M").to_string()
            }
            None => "".to_string(),
        });

        rows.push(Row::new(vec![name_cell, size_cell, date_cell]).style(text_style));
    }

    let widths = [
        Constraint::Percentage(55),
        Constraint::Percentage(15),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Name", "Size", "Date Modified"]).style(
                Style::default()
                    .fg(parse_color(&theme.header_fg))
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(block);

    f.render_widget(table, area);
}

/// Brief view: two columns of filenames side by side.
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

    // Split inner area into two equal columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let col_height = cols[0].height as usize;

    for col_idx in 0..2 {
        let start = col_idx * col_height;
        let col_entries: Vec<_> = panel
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(col_height)
            .collect();

        let mut rows = Vec::new();
        for (i, entry) in col_entries {
            let is_selected = panel.selected_paths.contains(&entry.path);
            let is_cursor = i == panel.cursor_index;
            let text_style = build_row_style(is_cursor, is_selected, is_active, theme);

            let display = if entry.is_dir && entry.name != ".." {
                format!("/{}", entry.name)
            } else {
                entry.name.clone()
            };
            rows.push(Row::new(vec![Cell::from(display)]).style(text_style));
        }

        let table =
            Table::new(rows, [Constraint::Percentage(100)]).block(Block::default().borders(Borders::NONE));
        f.render_widget(table, cols[col_idx]);
    }
}

/// Builds a cell style based on cursor/selection/active state.
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

/// Helper to format raw byte count into human units.
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
