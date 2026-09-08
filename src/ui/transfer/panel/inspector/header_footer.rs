use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Gauge, Paragraph};

use crate::app::state::TransferUIState;
use crate::fs::transfer::job::{TransferJobStatus, TransferProgress};
use crate::ui::transfer::panel::summarize_path;

pub(crate) fn render_header(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    prog: Option<&TransferProgress>,
    job: &crate::fs::transfer::job::TransferJob,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);

    let file_text = match prog {
        Some(p)
            if job.status == TransferJobStatus::Transferring
                || job.status == TransferJobStatus::Scanning
                || job.status == TransferJobStatus::Verifying =>
        {
            let path = std::path::Path::new(&p.current_file);
            format!("Current File: {}", summarize_path(path))
        }
        _ => format!("Job status: {:?}", job.status),
    };
    f.render_widget(
        Paragraph::new(file_text).style(Style::default().fg(Color::White)),
        chunks[0],
    );

    let percent = prog.map(|p| p.percent_bytes() as u16).unwrap_or(0);
    let label = match job.status {
        TransferJobStatus::Completed => "100% (Completed)".to_string(),
        TransferJobStatus::Failed => "Failed".to_string(),
        TransferJobStatus::Cancelled => "Cancelled".to_string(),
        _ => {
            if prog.is_some() {
                format!("{}%", percent)
            } else {
                "0%".to_string()
            }
        }
    };
    let speed_formatted = if prog.is_some()
        && (job.status == TransferJobStatus::Transferring
            || job.status == TransferJobStatus::Verifying)
    {
        bytesize::ByteSize(ts.speed_info.0 as u64).to_string()
    } else {
        "0 B".to_string()
    };
    let eta_text = match prog.and(ts.speed_info.1) {
        Some(secs)
            if job.status == TransferJobStatus::Transferring
                || job.status == TransferJobStatus::Verifying =>
        {
            format!("ETA {}s", secs)
        }
        _ => "ETA --".to_string(),
    };

    let gauge_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(30)])
        .split(chunks[1]);

    let gauge = Gauge::default()
        .percent(if job.status == TransferJobStatus::Completed {
            100
        } else {
            percent
        })
        .label(label)
        .gauge_style(
            Style::default()
                .fg(match job.status {
                    TransferJobStatus::Completed => Color::LightGreen,
                    TransferJobStatus::Cancelled => Color::Red,
                    TransferJobStatus::Failed => Color::LightRed,
                    TransferJobStatus::Paused => Color::Yellow,
                    _ => Color::Green,
                })
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(gauge, gauge_chunk[0]);

    let info_text = format!(" {}/s | {} ", speed_formatted, eta_text);
    f.render_widget(
        Paragraph::new(info_text).style(Style::default().fg(Color::Yellow)),
        gauge_chunk[1],
    );
}

pub(crate) fn render_footer(
    f: &mut Frame,
    area: Rect,
    _job: &crate::fs::transfer::job::TransferJob,
) {
    let footer_text =
        " [p] Pause/Resume  [s] Skip File  [x] Cancel Job  [Del] Remove Job  [Esc] Minimize ";
    let p = Paragraph::new(footer_text)
        .block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(p, area);
}
