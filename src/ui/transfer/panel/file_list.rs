use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};

use crate::app::state::TransferUIState;
use crate::fs::transfer::job::TransferResults;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};

pub(crate) fn render_file_list_tab(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    res: &TransferResults,
    theme: &crate::config::theme::Theme,
    scrollbar: Option<&crate::ui::scrollbar::ScrollbarUiState>,
) {
    let total_files = res.failed_files.len() + res.skipped_files.len() + res.completed_files.len();

    if total_files == 0 {
        let empty_p = Paragraph::new("\n No files transferred yet.").style(
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        );
        f.render_widget(empty_p, area);
        return;
    }

    let height = area.height.saturating_sub(3) as usize;
    let cursor = ts.file_list_cursor;

    let start = scrollbar::centered_scroll(cursor, total_files, height);
    let end = start + height.min(total_files.saturating_sub(start));

    let rows: Vec<Row> = (start..end)
        .map(|i| file_row(res, i, i == cursor))
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Min(30),
            Constraint::Length(12),
            Constraint::Length(15),
        ],
    )
    .header(
        Row::new(vec!["Status", "File Path", "Size", "Hashes"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    )
    .row_highlight_style(Style::default());

    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(Some(cursor.saturating_sub(start)));

    f.render_stateful_widget(table, area, &mut table_state);

    scrollbar::render_vertical_inside_block(
        f,
        area,
        ScrollView {
            content_len: total_files,
            viewport_len: height.max(1),
            offset: start,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::TransferFiles,
        },
    );
}

/// Row `i` of the list: failed files first, then skipped, then completed,
/// colored red/yellow/green (inverted when selected).
fn file_row(res: &TransferResults, i: usize, selected: bool) -> Row<'static> {
    let f_len = res.failed_files.len();
    let s_len = res.skipped_files.len();
    let (cells, color, selected_fg) = if i < f_len {
        let f = &res.failed_files[i];
        let cells = [
            " ✗ FAIL ".to_string(),
            f.src.to_string_lossy().into_owned(),
            "-".to_string(),
            f.error.clone(),
        ];
        (cells, Color::Red, Color::White)
    } else if i < f_len + s_len {
        let f = &res.skipped_files[i - f_len];
        let cells = [
            " ⚠ SKIP ".to_string(),
            f.src.to_string_lossy().into_owned(),
            "-".to_string(),
            f.reason.clone(),
        ];
        (cells, Color::Yellow, Color::Black)
    } else {
        let f = &res.completed_files[i - f_len - s_len];
        let src_hash = f.src_hash.as_deref().unwrap_or("-");
        let dst_hash = f.dst_hash.as_deref().unwrap_or("-");
        let hash_text = format!(
            "{} : {}",
            &src_hash[..src_hash.len().min(4)],
            &dst_hash[..dst_hash.len().min(4)]
        );
        let cells = [
            " ✓ OK ".to_string(),
            f.src.to_string_lossy().into_owned(),
            bytesize::ByteSize(f.size).to_string(),
            hash_text,
        ];
        (cells, Color::Green, Color::Black)
    };
    let style = if selected {
        Style::default()
            .fg(selected_fg)
            .bg(color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    };
    Row::new(cells).style(style)
}
