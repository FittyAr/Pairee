use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::ui::panel::helpers::{build_row_style, entry_display_name, visible_range};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Block, Cell, Row, Table},
};

pub(crate) fn render_wide(
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
