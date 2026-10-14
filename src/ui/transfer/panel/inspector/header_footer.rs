use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Gauge, Paragraph};

use crate::app::state::TransferUIState;
use crate::config::localization::t;
use crate::fs::transfer::job::{TransferJobStatus, TransferProgress};
use crate::ui::transfer::labels;
use crate::ui::transfer::panel::summarize_path;

pub(crate) fn render_header(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    prog: Option<&TransferProgress>,
    job: &crate::fs::transfer::job::TransferJob,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
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
            t("transfer_current_file").replace("{}", &summarize_path(path))
        }
        _ => t("transfer_job_status").replace("{}", &labels::status(job)),
    };
    f.render_widget(
        Paragraph::new(file_text).style(Style::default().fg(theme.ui.strong)),
        chunks[0],
    );

    let percent = prog.map(|p| p.percent_bytes() as u16).unwrap_or(0);
    let label = match job.status {
        TransferJobStatus::Completed => t("transfer_gauge_completed"),
        TransferJobStatus::Failed | TransferJobStatus::Cancelled => labels::status(job),
        _ => format!("{percent}%"),
    };
    let moving = prog.is_some()
        && (job.status == TransferJobStatus::Transferring
            || job.status == TransferJobStatus::Verifying);
    let speed = if moving { ts.speed_info.0 as u64 } else { 0 };
    let speed_formatted = bytesize::ByteSize(speed).to_string();
    let eta_text = labels::eta(ts.speed_info.1.filter(|_| moving));

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
                    // A running bar is green whatever the stage.
                    TransferJobStatus::Queued
                    | TransferJobStatus::Scanning
                    | TransferJobStatus::Verifying => theme.ui.success,
                    status => crate::ui::transfer::status_color(theme, status),
                })
                .bg(theme.ui.muted)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(gauge, gauge_chunk[0]);

    let info_text = format!(" {}/s | {} ", speed_formatted, eta_text);
    f.render_widget(
        Paragraph::new(info_text).style(Style::default().fg(theme.ui.warning)),
        gauge_chunk[1],
    );
}

pub(crate) fn render_footer(
    f: &mut Frame,
    area: Rect,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    let p = Paragraph::new(t("transfer_footer_keys"))
        .block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(theme.ui.muted)),
        )
        .style(
            Style::default()
                .fg(theme.ui.accent)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(p, area);
}
