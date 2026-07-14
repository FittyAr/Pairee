use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Table,
};

use crate::app::context::AppContext;
use crate::app::state::{AppState, TransferTab, TransferUIState, TransferViewMode};
use crate::config::localization::t;
use crate::fs::transfer::job::{TransferJobStatus, TransferProgress, TransferResults};
use crate::ui::popup::centered_rect;

pub fn render_transfer_panel(f: &mut Frame, state: &AppState, context: &AppContext) {
    let transfer_state = match &state.transfer {
        Some(ts) => ts,
        None => return,
    };

    if transfer_state.view_mode != TransferViewMode::Expanded {
        return;
    }

    let size = f.area();
    // Popup centrado: 80% ancho, 75% alto
    let popup_area = centered_rect(80, 75, size);

    // Clear para tapar los paneles debajo
    f.render_widget(Clear, popup_area);

    // Contenedor principal con borde redondeado
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" {} ", t("transfer_title")));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Dividir horizontalmente: Sidebar (30%) e Inspector (70%)
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(inner_area);

    let jobs = transfer_state.engine.queue.get_all();

    // --- 1. RENDER SIDEBAR (COL IZQUIERDA) ---
    render_jobs_sidebar(f, main_layout[0], transfer_state, &jobs);

    // --- 2. RENDER INSPECTOR (COL DERECHA) ---
    if jobs.is_empty() {
        let empty_p =
            Paragraph::new("\n No jobs in queue.\n Press F5 to Copy or F6 to Move files.")
                .style(
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                )
                .block(
                    Block::default()
                        .borders(Borders::LEFT)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );
        f.render_widget(empty_p, main_layout[1]);
    } else {
        let cursor_idx = transfer_state
            .queue_cursor
            .min(jobs.len().saturating_sub(1));
        if let Some(selected_job) = jobs.get(cursor_idx) {
            let inspector_area = main_layout[1];
            // Aseguramos una división vertical del inspector con borde izquierdo
            let inspector_block = Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(Color::DarkGray));
            let inner_inspector = inspector_block.inner(inspector_area);
            f.render_widget(inspector_block, inspector_area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(4), // Cabecera
                    Constraint::Length(3), // Pestañas
                    Constraint::Min(5),    // Contenido
                    Constraint::Length(3), // Footer (Acciones)
                ])
                .split(inner_inspector);

            let progress = selected_job.progress.as_ref();
            let results = &selected_job.results;
            let log_lines = &selected_job.log_lines;

            // Header
            render_header(f, chunks[0], transfer_state, progress, selected_job);

            // Tabs
            render_tabs(
                f,
                chunks[1],
                transfer_state.active_tab,
                &context.config.theme,
            );

            // Content
            match transfer_state.active_tab {
                TransferTab::FileList => {
                    render_file_list_tab(f, chunks[2], transfer_state, results)
                }
                TransferTab::Options => {
                    render_options_tab(f, chunks[2], transfer_state, selected_job)
                }
                TransferTab::Status => {
                    render_status_tab(f, chunks[2], transfer_state, progress, results)
                }
                TransferTab::Log => render_log_tab(f, chunks[2], log_lines),
            }

            // Footer
            render_footer(f, chunks[3], selected_job);
        }
    }
}

fn render_jobs_sidebar(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    jobs: &[crate::fs::transfer::job::TransferJob],
) {
    let mut list_items = Vec::new();
    for (idx, job) in jobs.iter().enumerate() {
        let is_selected = idx == ts.queue_cursor;

        let op_name = match job.operation {
            crate::fs::transfer::job::TransferOperation::Copy => "Copy",
            crate::fs::transfer::job::TransferOperation::Move => "Move",
            crate::fs::transfer::job::TransferOperation::Delete => "Delete",
        };

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
                .unwrap_or(&job.destination.as_os_str())
                .to_string_lossy()
        );

        let mut item_style = Style::default().fg(color);
        if is_selected {
            item_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
        }

        let mut lines = vec![
            ratatui::text::Line::from(ratatui::text::Span::styled(title_line, item_style)),
            ratatui::text::Line::from(ratatui::text::Span::styled(
                dest_str,
                if is_selected {
                    Style::default().fg(Color::Black)
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
        .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black));

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(ts.queue_cursor));

    f.render_stateful_widget(jobs_list, area, &mut list_state);
}

fn render_header(
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
    let eta_text = match prog.and_then(|_| ts.speed_info.1) {
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
                .fg(if job.status == TransferJobStatus::Completed {
                    Color::LightGreen
                } else {
                    Color::Green
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

fn render_tabs(
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

fn render_file_list_tab(f: &mut Frame, area: Rect, ts: &TransferUIState, res: &TransferResults) {
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

    let start = if cursor > height / 2 {
        cursor.saturating_sub(height / 2)
    } else {
        0
    };
    let start = if start + height > total_files {
        total_files.saturating_sub(height)
    } else {
        start
    };
    let end = start + height.min(total_files.saturating_sub(start));

    let mut rows = Vec::new();
    let f_len = res.failed_files.len();
    let s_len = res.skipped_files.len();

    for i in start..end {
        let is_selected = i == cursor;
        let mut style = if is_selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        if i < f_len {
            let f = &res.failed_files[i];
            if !is_selected {
                style = style.fg(Color::Red);
            }
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
            if !is_selected {
                style = style.fg(Color::Yellow);
            }
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
            if !is_selected {
                style = style.fg(Color::Green);
            }
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
}

fn render_options_tab(
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

fn render_status_tab(
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
            format!(
                "{}/s",
                bytesize::ByteSize(ts.speed_info.0 as u64).to_string()
            ),
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

fn render_log_tab(f: &mut Frame, area: Rect, log_lines: &[String]) {
    let items: Vec<ListItem> = log_lines
        .iter()
        .rev()
        .take(15)
        .map(|line| ListItem::new(line.as_str()).style(Style::default().fg(Color::Gray)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Event Log ")
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect, _job: &crate::fs::transfer::job::TransferJob) {
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

pub fn summarize_path(path: &std::path::Path) -> String {
    if let Some(file_name) = path.file_name() {
        if let Some(parent) = path.parent() {
            if let Some(parent_name) = parent.file_name() {
                let sep = std::path::MAIN_SEPARATOR;
                return format!(
                    "..{}{}{}{}",
                    sep,
                    parent_name.to_string_lossy(),
                    sep,
                    file_name.to_string_lossy()
                );
            }
        }
        return file_name.to_string_lossy().into_owned();
    }
    path.to_string_lossy().into_owned()
}
