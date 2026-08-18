use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Row, Table};

use crate::app::state::TransferUIState;
use crate::fs::transfer::job::TransferResults;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};

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

    let mut rows = Vec::new();
    let f_len = res.failed_files.len();
    let s_len = res.skipped_files.len();

    for i in start..end {
        let is_selected = i == cursor;
        let style = if is_selected {
            let (fg_sel, bg_sel) = if i < f_len {
                (Color::White, Color::Red)
            } else if i < f_len + s_len {
                (Color::Black, Color::Yellow)
            } else {
                (Color::Black, Color::Green)
            };
            Style::default()
                .fg(fg_sel)
                .bg(bg_sel)
                .add_modifier(Modifier::BOLD)
        } else {
            if i < f_len {
                Style::default().fg(Color::Red)
            } else if i < f_len + s_len {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Green)
            }
        };

        if i < f_len {
            let f = &res.failed_files[i];
            rows.push(
                Row::new(vec![
                    " ✗ FAIL ".to_string(),
                    f.src.to_string_lossy().into_owned(),
                    "-".to_string(),
                    f.error.clone(),
                ])
                .style(style),
            );
        } else if i < f_len + s_len {
            let f = &res.skipped_files[i - f_len];
            rows.push(
                Row::new(vec![
                    " ⚠ SKIP ".to_string(),
                    f.src.to_string_lossy().into_owned(),
                    "-".to_string(),
                    f.reason.clone(),
                ])
                .style(style),
            );
        } else {
            let f = &res.completed_files[i - f_len - s_len];
            let src_hash = f.src_hash.as_deref().unwrap_or("-");
            let dst_hash = f.dst_hash.as_deref().unwrap_or("-");
            let hash_text = format!(
                "{} : {}",
                &src_hash[..src_hash.len().min(4)],
                &dst_hash[..dst_hash.len().min(4)]
            );

            rows.push(
                Row::new(vec![
                    " ✓ OK ".to_string(),
                    f.src.to_string_lossy().into_owned(),
                    bytesize::ByteSize(f.size).to_string(),
                    hash_text,
                ])
                .style(style),
            );
        }
    }

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
        total_files,
        height.max(1),
        start,
        theme,
        ScrollbarSurface::Popup,
        scrollbar,
        ScrollTargetId::TransferFiles,
    );
}
