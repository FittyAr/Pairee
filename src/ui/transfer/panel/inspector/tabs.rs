use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};

use crate::app::state::{TransferTab, TransferUIState};
use crate::fs::transfer::job::TransferResults;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};

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
            Constraint::Length(1),
            Constraint::Length(1),
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
    prog: Option<&crate::fs::transfer::job::TransferProgress>,
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
