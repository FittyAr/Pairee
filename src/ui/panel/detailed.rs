use crate::app::context::AppContext;
use crate::app::state::PanelState;
use crate::config::localization::t;
use crate::fs::attrs::format_unix_mode;
use crate::ui::panel::helpers::{
    build_row_style, entry_display_name, format_file_size, visible_range,
};
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Cell, Row, Table},
};

pub(crate) fn render_detailed(
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
            let is_dimmed = if let Some(ref mask) = panel.quick_filter_mask {
                if entry.name == ".." {
                    false
                } else {
                    !entry.name.to_lowercase().contains(&mask.to_lowercase())
                }
            } else {
                false
            };
            let style = build_row_style(
                entry,
                i == panel.cursor_index,
                panel.selected_paths.contains(&entry.path),
                is_active,
                theme,
                highlight_files,
                is_dimmed,
            );
            let (perm_str, owner) = if panel.ssh_conn.is_some() {
                ("?????????".to_string(), "?".to_string())
            } else if let Ok(attrs) = crate::fs::attrs::read_attrs(&entry.path) {
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

    let header = Row::new(vec![
        t("col_name"),
        t("col_perms"),
        t("col_owner"),
        t("col_size"),
    ])
    .style(
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
