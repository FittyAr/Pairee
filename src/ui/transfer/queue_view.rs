use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Borders, BorderType, Row, Table};
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::app::state::TransferUIState;

pub fn render_queue_view(f: &mut Frame, area: Rect, ts: &TransferUIState) {
    // Tomar snapshot de la cola de trabajos
    let jobs = ts.engine.queue.get_all();

    if jobs.is_empty() {
        let empty_p = ratatui::widgets::Paragraph::new("\n No jobs in queue.")
            .style(Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC));
        f.render_widget(empty_p, area);
        return;
    }

    let mut rows = Vec::new();
    for (idx, job) in jobs.iter().enumerate() {
        let is_selected = idx == ts.queue_cursor;
        let mut row_style = if is_selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let status_str = match job.status {
            crate::fs::transfer::job::TransferJobStatus::Queued => " Queued ",
            crate::fs::transfer::job::TransferJobStatus::Scanning => " Scanning ",
            crate::fs::transfer::job::TransferJobStatus::Transferring => " Transferring ",
            crate::fs::transfer::job::TransferJobStatus::Verifying => " Verifying ",
            crate::fs::transfer::job::TransferJobStatus::Paused => " Paused ",
            crate::fs::transfer::job::TransferJobStatus::Completed => " Completed ",
            crate::fs::transfer::job::TransferJobStatus::Failed => " Failed ",
            crate::fs::transfer::job::TransferJobStatus::Cancelled => " Cancelled ",
        };

        let status_color = match job.status {
            crate::fs::transfer::job::TransferJobStatus::Queued => Color::Gray,
            crate::fs::transfer::job::TransferJobStatus::Scanning => Color::Blue,
            crate::fs::transfer::job::TransferJobStatus::Transferring => Color::Green,
            crate::fs::transfer::job::TransferJobStatus::Verifying => Color::LightBlue,
            crate::fs::transfer::job::TransferJobStatus::Paused => Color::Yellow,
            crate::fs::transfer::job::TransferJobStatus::Completed => Color::Green,
            crate::fs::transfer::job::TransferJobStatus::Failed => Color::Red,
            crate::fs::transfer::job::TransferJobStatus::Cancelled => Color::DarkGray,
        };

        let sources_count = job.sources.len();
        let sources_desc = if sources_count == 1 {
            job.sources[0].to_string_lossy().into_owned()
        } else {
            format!("{} items", sources_count)
        };

        let operation_str = match job.operation {
            crate::fs::transfer::job::TransferOperation::Copy => "Copy",
            crate::fs::transfer::job::TransferOperation::Move => "Move",
        };

        rows.push(Row::new(vec![
            format!(" #{} ", idx + 1),
            operation_str.to_string(),
            sources_desc,
            job.destination.to_string_lossy().into_owned(),
            status_str.to_string(),
        ]).style(if is_selected { row_style } else { Style::default().fg(status_color) }));
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),  // Id
            Constraint::Length(10), // Operacion
            Constraint::Min(25),    // Origen
            Constraint::Min(25),    // Destino
            Constraint::Length(15), // Estado
        ]
    )
    .header(Row::new(vec!["Job", "Op", "Source", "Destination", "Status"]).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
    .block(Block::default().borders(Borders::ALL).title(" Jobs Queue ").border_type(BorderType::Rounded));

    f.render_widget(table, area);
}
