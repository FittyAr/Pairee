use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph};

use crate::app::state::{TransferTab, TransferUIState};
use crate::fs::transfer::job::{TransferJobStatus, TransferProgress, TransferResults};
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};
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

pub(crate) fn render_tabs(
    f: &mut Frame,
    area: Rect,
    active_tab: TransferTab,
    theme: &crate::config::theme::Theme,
) {
    let tab_titles = vec![
        (0, "[1] File List"),
        (1, "[2] Options"),
        (2, "[3] Status"),
        (3, "[4] Log"),
    ];

    let tab_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tab text row
            Constraint::Length(1), // Tab separator line
            Constraint::Min(0),
        ])
        .split(area);

    let tab_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(tab_layout[0]);

    // Render bottom separator line using theme border color
    let border_color = crate::ui::theme_apply::parse_color(&theme.popup_border);
    f.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(Style::default().fg(border_color)),
        tab_layout[1],
    );

    let fg_color = crate::ui::theme_apply::parse_color(&theme.popup_fg);

    for (idx, (tab_idx, title)) in tab_titles.into_iter().enumerate() {
        let is_active = tab_idx == active_tab as usize;

        let text = if is_active {
            format!("▶ {} ◀", title)
        } else {
            format!("  {}  ", title)
        };

        let mut style = Style::default().fg(fg_color);
        if is_active {
            style = style.add_modifier(Modifier::BOLD);
        }

        let p = Paragraph::new(text)
            .style(style)
            .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(p, tab_chunks[idx]);
    }
}

pub(crate) fn render_options_tab(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    job: &crate::fs::transfer::job::TransferJob,
) {
    let options = &job.options;

    let opt_labels = vec![
        format!(
            "Direct I/O (bypass cache): {}",
            if options.direct_io { "Yes" } else { "No" }
        ),
        format!(
            "Verify integrity after transfer: {}",
            if options.verify_after_copy {
                "Yes"
            } else {
                "No"
            }
        ),
        format!(
            "Preserve timestamps (created, modified): {}",
            if options.preserve_timestamps {
                "Yes"
            } else {
                "No"
            }
        ),
        format!(
            "Preserve attributes and permissions: {}",
            if options.preserve_attributes {
                "Yes"
            } else {
                "No"
            }
        ),
        format!("Post-Action (On Finish): {:?}", ts.post_action),
        format!(
            "Buffer size: {}",
            bytesize::ByteSize(options.buffer_size.to_bytes() as u64).to_string()
        ),
        format!("Hash algorithm: {}", options.hash_algorithm.as_str()),
        format!(
            "Preserve Security / ACLs: {}",
            if options.preserve_acl { "Yes" } else { "No" }
        ),
        format!(
            "Preserve Alternate Data Streams: {}",
            if options.preserve_streams {
                "Yes"
            } else {
                "No"
            }
        ),
        format!(
            "Skip symbolic links: {}",
            if options.skip_symlinks { "Yes" } else { "No" }
        ),
        format!(
            "Follow symbolic links: {}",
            if options.follow_symlinks { "Yes" } else { "No" }
        ),
        format!(
            "Limit bandwidth: {}",
            if let Some(rate) = options.limit_bandwidth_rate {
                format!("{} /s", bytesize::ByteSize(rate))
            } else {
                "No limit".to_string()
            }
        ),
    ];

    let mut lines = Vec::new();
    lines.push(ratatui::text::Line::from(""));
    for (idx, label) in opt_labels.iter().enumerate() {
        let is_selected = idx == ts.options_cursor;
        if is_selected {
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("  ▶  {}  ", label),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("     {}  ", label),
                Style::default().fg(Color::Gray),
            )));
        }
        lines.push(ratatui::text::Line::from(""));
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Active Transfer Settings (Use Up/Down + Enter to toggle) ")
            .border_type(BorderType::Rounded),
    );
    f.render_widget(p, area);
}

pub(crate) fn render_status_tab(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    prog: Option<&TransferProgress>,
    res: &TransferResults,
) {
    let (
        files_total,
        files_completed,
        files_failed,
        files_skipped,
        bytes_total,
        bytes_transferred,
        speed,
        eta,
    ) = match prog {
        Some(p) => (
            p.files_total,
            p.files_completed,
            p.files_failed,
            p.files_skipped,
            bytesize::ByteSize(p.bytes_total).to_string(),
            bytesize::ByteSize(p.bytes_transferred).to_string(),
            format!("{}/s", bytesize::ByteSize(ts.speed_info.0 as u64)),
            match ts.speed_info.1 {
                Some(secs) => format!("{} seconds", secs),
                None => "Calculating...".to_string(),
            },
        ),
        None => {
            let completed = res.completed_files.len();
            let failed = res.failed_files.len();
            let skipped = res.skipped_files.len();
            let total = completed + failed + skipped;
            let bytes: u64 = res.completed_files.iter().map(|f| f.size).sum();
            (
                total,
                completed,
                failed,
                skipped,
                bytesize::ByteSize(bytes).to_string(),
                bytesize::ByteSize(bytes).to_string(),
                "0 B/s".to_string(),
                "Finished".to_string(),
            )
        }
    };

    let text = format!(
        r#"  - Total Files: {}
  - Files Completed: {}
  - Files Failed: {}
  - Files Skipped: {}
  
  - Total Size: {}
  - Bytes Copied: {}
  - Current Speed: {}
  - Estimated Time (ETA): {}"#,
        files_total,
        files_completed,
        files_failed,
        files_skipped,
        bytes_total,
        bytes_transferred,
        speed,
        eta
    );

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Statistics ")
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(p, area);
}

pub(crate) fn render_log_tab(
    f: &mut Frame,
    area: Rect,
    log_lines: &[String],
    theme: &crate::config::theme::Theme,
    ts: &TransferUIState,
    scrollbar: Option<&crate::ui::scrollbar::ScrollbarUiState>,
) {
    let viewport = area.height.saturating_sub(2) as usize;
    let total = log_lines.len();
    let max_start = total.saturating_sub(viewport);
    // Prefer user scroll; default to following the end.
    let start = if ts.log_scroll > max_start {
        max_start
    } else if total > viewport {
        ts.log_scroll
    } else {
        0
    };

    let items: Vec<ListItem> = log_lines
        .iter()
        .skip(start)
        .take(viewport)
        .map(|line| ListItem::new(line.as_str()).style(Style::default().fg(Color::Gray)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Event Log ")
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, area);

    scrollbar::render_vertical_inside_block(
        f,
        area,
        total,
        viewport.max(1),
        start,
        theme,
        ScrollbarSurface::Popup,
        scrollbar,
        ScrollTargetId::TransferLog,
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
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(p, area);
}
