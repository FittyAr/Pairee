use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph};

use crate::app::state::{TransferTab, TransferUIState};
use crate::config::localization::t;
use crate::fs::transfer::job::TransferResults;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};
use crate::ui::transfer::labels::{post_action, yes_no};

pub(crate) fn render_tabs(
    f: &mut Frame,
    area: Rect,
    active_tab: TransferTab,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    let tab_titles = [
        (0, t("transfer_tab_files")),
        (1, t("transfer_tab_options")),
        (2, t("transfer_tab_status")),
        (3, t("transfer_tab_log")),
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

    let border_color = theme.popup_border;
    f.render_widget(
        crate::ui::popup::kit::separator(area.width, crate::ui::popup::kit::fg(border_color)),
        tab_layout[1],
    );

    let fg_color = theme.popup_fg;

    for (idx, (tab_idx, title)) in tab_titles.into_iter().enumerate() {
        // Digits 1-4 open the tabs.
        crate::ui::popup::kit::mouse::keys(
            tab_chunks[idx],
            crate::ui::popup::kit::mouse::key(crossterm::event::KeyCode::Char(char::from(
                b'1' + idx as u8,
            ))),
        );
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
    theme: &crate::ui::resolved_theme::ResolvedTheme,
) {
    use crate::app::state::transfer_options::OptionRow;
    let options = &job.options;
    // Once the job runs only the live rows still change it.
    let started = job.status != crate::fs::transfer::job::TransferJobStatus::Queued;
    let opt_labels = OptionRow::ALL.map(|row| {
        let label = t(option_key(row)).replace("{}", &option_value(row, options, ts));
        if started && !row.is_live() {
            format!("{label}  {}", t("transfer_opt_fixed"))
        } else {
            label
        }
    });

    let mut lines = Vec::new();
    lines.push(ratatui::text::Line::from(""));
    for (idx, label) in opt_labels.iter().enumerate() {
        let is_selected = idx == ts.options_cursor;
        if is_selected {
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("  ▶  {}  ", label),
                Style::default()
                    .fg(theme.ui.badge_fg)
                    .bg(theme.ui.accent)
                    .add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(ratatui::text::Line::from(ratatui::text::Span::styled(
                format!("     {}  ", label),
                Style::default().fg(theme.ui.secondary),
            )));
        }
        lines.push(ratatui::text::Line::from(""));
    }

    // A blank line, then two lines per option: keep the selected one shown.
    let selected_line = 1 + 2 * ts.options_cursor;
    let visible = area.height.saturating_sub(2) as usize;
    let top = scrollbar::follow_scroll(selected_line, 0, visible);
    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t("transfer_options_title"))
                .border_type(BorderType::Rounded),
        )
        .scroll((u16::try_from(top).unwrap_or(u16::MAX), 0));
    f.render_widget(p, area);
}

/// Translation key of the label of an Options row.
fn option_key(row: crate::app::state::transfer_options::OptionRow) -> &'static str {
    use crate::app::state::transfer_options::OptionRow as R;
    match row {
        R::DirectIo => "transfer_opt_direct_io",
        R::Verify => "transfer_opt_verify",
        R::Timestamps => "transfer_opt_timestamps",
        R::Attributes => "transfer_opt_attributes",
        R::PostAction => "transfer_opt_post_action",
        R::Buffer => "transfer_opt_buffer",
        R::Hash => "transfer_opt_hash",
        R::Acl => "transfer_opt_acl",
        R::Streams => "transfer_opt_streams",
        R::SkipLinks => "transfer_opt_skip_links",
        R::FollowLinks => "transfer_opt_follow_links",
        R::Bandwidth => "transfer_opt_bandwidth",
        R::Conflict => "transfer_opt_conflict",
    }
}

/// The value an Options row shows.
fn option_value(
    row: crate::app::state::transfer_options::OptionRow,
    o: &crate::fs::transfer::options::TransferOptions,
    ts: &TransferUIState,
) -> String {
    use crate::app::state::transfer_options::OptionRow as R;
    match row {
        R::DirectIo => yes_no(o.direct_io),
        R::Verify => yes_no(o.verify_after_copy),
        R::Timestamps => yes_no(o.preserve_timestamps),
        R::Attributes => yes_no(o.preserve_attributes),
        R::PostAction => post_action(&ts.post_action),
        R::Buffer => bytesize::ByteSize(o.buffer_size.to_bytes() as u64).to_string(),
        R::Hash => o.hash_algorithm.as_str().to_string(),
        R::Acl => yes_no(o.preserve_acl),
        R::Streams => yes_no(o.preserve_streams),
        R::SkipLinks => yes_no(o.skip_symlinks),
        R::FollowLinks => yes_no(o.follow_symlinks),
        R::Bandwidth => match o.limit_bandwidth_rate {
            Some(rate) => format!("{} /s", bytesize::ByteSize(rate)),
            None => t("transfer_no_limit"),
        },
        R::Conflict => match o.conflict_resolution.as_str() {
            "overwrite" => t("transfer_conflict_mode_overwrite"),
            "overwrite_older" => t("transfer_conflict_mode_overwrite_older"),
            "skip" => t("transfer_conflict_mode_skip"),
            "rename" => t("transfer_conflict_mode_rename"),
            "keep_both" => t("transfer_conflict_mode_keep_both"),
            // Anything else asks (see `ConflictResolver`).
            _ => t("transfer_conflict_mode_ask"),
        },
    }
}

pub(crate) fn render_status_tab(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    prog: Option<&crate::fs::transfer::job::TransferProgress>,
    res: &TransferResults,
    theme: &crate::ui::resolved_theme::ResolvedTheme,
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
                Some(secs) => t("transfer_seconds").replace("{}", &secs.to_string()),
                None => t("transfer_calculating"),
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
                format!("{}/s", bytesize::ByteSize(0)),
                t("transfer_finished"),
            )
        }
    };

    let text = t("transfer_stats_body")
        .replace("{total}", &files_total.to_string())
        .replace("{completed}", &files_completed.to_string())
        .replace("{failed}", &files_failed.to_string())
        .replace("{skipped}", &files_skipped.to_string())
        .replace("{size}", &bytes_total)
        .replace("{copied}", &bytes_transferred)
        .replace("{speed}", &speed)
        .replace("{eta}", &eta);

    let p = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(t("transfer_stats_title"))
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(theme.ui.accent));
    f.render_widget(p, area);
}

pub(crate) fn render_log_tab(
    f: &mut Frame,
    area: Rect,
    log_lines: &[String],
    theme: &crate::ui::resolved_theme::ResolvedTheme,
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
        .map(|line| ListItem::new(line.as_str()).style(Style::default().fg(theme.ui.secondary)))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(t("transfer_log_title"))
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, area);

    scrollbar::render_vertical_inside_block(
        f,
        area,
        ScrollView {
            content_len: total,
            viewport_len: viewport.max(1),
            offset: start,
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::TransferLog,
        },
    );
}
