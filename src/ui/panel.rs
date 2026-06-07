use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub fn render_panel(
    f: &mut Frame,
    area: Rect,
    panel: &PanelState,
    is_active: bool,
    context: &AppContext,
) {
    let theme = &context.config.theme;

    // 1. Determine borders colors based on keyboard focus status
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

    // 2. Map file list into table Rows
    let mut rows = Vec::new();
    for (i, entry) in panel.entries.iter().enumerate() {
        let is_selected = panel.selected_paths.contains(&entry.path);
        let is_cursor = i == panel.cursor_index;

        let mut text_style = Style::default().fg(parse_color(&theme.panel_fg));

        if is_selected {
            text_style = text_style.fg(parse_color(&theme.marked_fg));
        }

        if is_cursor && is_active {
            text_style = text_style
                .bg(parse_color(&theme.selection_bg))
                .fg(parse_color(&theme.selection_fg))
                .add_modifier(Modifier::BOLD);
        } else if is_cursor && !is_active {
            // Subtle selection cursor for background panel
            text_style = text_style.bg(parse_color("DarkGray"));
        }

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

    // 3. Define responsive column percentages
    let widths = [
        ratatui::layout::Constraint::Percentage(55),
        ratatui::layout::Constraint::Percentage(15),
        ratatui::layout::Constraint::Percentage(30),
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
