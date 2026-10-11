use super::list_ctx::ListCtx;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Cell, Row, Table},
};

pub(crate) fn render_brief(f: &mut Frame, area: Rect, block: Block, ctx: &ListCtx) {
    let panel = ctx.panel;
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let col_height = cols[0].height.saturating_sub(2) as usize;
    let (start, _) = ctx.range(col_height * 2);

    for col_idx in 0..2usize {
        let col_start = start + col_idx * col_height;
        let rows: Vec<Row> = panel
            .entries
            .iter()
            .enumerate()
            .skip(col_start)
            .take(col_height)
            .map(|(i, entry)| {
                let style = ctx.row_style(i, entry);
                let name_width = cols[col_idx].width.saturating_sub(2) as usize;
                Row::new(vec![Cell::from(ctx.name(entry, Some(name_width.max(4))))]).style(style)
            })
            .collect();

        let table = Table::new(rows, [Constraint::Percentage(100)])
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(table, cols[col_idx]);
        let shown = Rect {
            height: col_height as u16,
            ..cols[col_idx]
        };
        ctx.register_rows(shown, col_start);
    }
}
