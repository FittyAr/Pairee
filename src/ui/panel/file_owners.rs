use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::config::localization::t;
use crate::ui::panel::helpers::{build_row_style, entry_display_name, visible_range};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Cell, Row, Table},
};

pub(crate) fn render_file_owners(
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
            let owner = if panel.ssh_conn.is_some() {
                "?".to_string()
            } else {
                crate::fs::attrs::read_attrs(&entry.path)
                    .map(|a| a.owner)
                    .unwrap_or_else(|_| "?".to_string())
            };
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
