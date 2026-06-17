use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::ui::panel::helpers::{build_row_style, entry_display_name};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub(crate) fn render_brief(
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
