use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem};

use crate::app::state::TransferUIState;
use crate::fs::transfer::job::TransferJobStatus;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};

pub(crate) fn render_jobs_sidebar(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    jobs: &[crate::fs::transfer::job::TransferJob],
    theme: &crate::config::theme::Theme,
    scrollbar: Option<&crate::ui::scrollbar::ScrollbarUiState>,
) {
    let mut list_items = Vec::new();
    let selected_job = jobs.get(ts.queue_cursor);
    let (sel_bg, sel_fg) = match selected_job.map(|j| &j.status) {
        Some(TransferJobStatus::Cancelled) => (Color::Red, Color::White),
        Some(TransferJobStatus::Failed) => (Color::LightRed, Color::White),
        Some(TransferJobStatus::Paused) => (Color::Yellow, Color::Black),
        Some(TransferJobStatus::Completed) => (Color::Green, Color::Black),
        _ => (Color::Cyan, Color::Black),
    };

    for (idx, job) in jobs.iter().enumerate() {
        let is_selected = idx == ts.queue_cursor;

        let op_name = job.operation.label();

        let (status_str, color) = match job.status {
            TransferJobStatus::Queued => ("Queued".to_string(), Color::Gray),
            TransferJobStatus::Scanning => ("Scanning...".to_string(), Color::Cyan),
            TransferJobStatus::Transferring => {
                let pct = job
                    .progress
                    .as_ref()
                    .map(|p| p.percent_bytes())
                    .unwrap_or(0.0);
                (format!("Running ({:.0}%)", pct), Color::Green)
            }
            TransferJobStatus::Verifying => ("Verifying...".to_string(), Color::LightBlue),
            TransferJobStatus::Paused => ("Paused".to_string(), Color::Yellow),
            TransferJobStatus::Completed => ("Completed".to_string(), Color::LightGreen),
            TransferJobStatus::Failed => ("Failed".to_string(), Color::LightRed),
            TransferJobStatus::Cancelled => ("Cancelled".to_string(), Color::Red),
        };

        let title_line = format!("#{} {} - {}", idx + 1, op_name, status_str);
        let dest_str = format!(
            "Dest: {}",
            job.destination
                .file_name()
                .unwrap_or(job.destination.as_os_str())
                .to_string_lossy()
        );

        let mut item_style = Style::default().fg(color);
        if is_selected {
            item_style = Style::default()
                .fg(sel_fg)
                .bg(sel_bg)
                .add_modifier(Modifier::BOLD);
        }

        let mut lines = vec![
            ratatui::text::Line::from(ratatui::text::Span::styled(title_line, item_style)),
            ratatui::text::Line::from(ratatui::text::Span::styled(
                dest_str,
                if is_selected {
                    Style::default().fg(sel_fg).bg(sel_bg)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            )),
            ratatui::text::Line::from(""),
        ];

        if idx == jobs.len() - 1 {
            lines.pop();
        }

        list_items.push(ListItem::new(lines));
    }

    let jobs_list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Jobs List ")
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(Style::default().bg(sel_bg).fg(sel_fg));

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(ts.queue_cursor));

    f.render_stateful_widget(jobs_list, area, &mut list_state);

    // Jobs list items are multi-line (~3 rows each); approximate viewport by area height.
    let viewport = area.height.saturating_sub(2) as usize;
    let offset = scrollbar::centered_scroll(ts.queue_cursor, jobs.len(), viewport.max(1));
    scrollbar::render_vertical_inside_block(
        f,
        area,
        jobs.len(),
        viewport.max(1),
        offset,
        theme,
        ScrollbarSurface::Popup,
        scrollbar,
        ScrollTargetId::TransferJobs,
    );
}
