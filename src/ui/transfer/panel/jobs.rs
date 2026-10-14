use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem};

use crate::app::state::TransferUIState;
use crate::config::localization::t;
use crate::fs::transfer::job::TransferJobStatus;
use crate::ui::scrollbar::{self, ScrollTargetId, ScrollbarSurface};
use crate::ui::scrollbar::{ScrollTarget, ScrollView};

pub(crate) fn render_jobs_sidebar(
    f: &mut Frame,
    area: Rect,
    ts: &TransferUIState,
    jobs: &[crate::fs::transfer::job::TransferJob],
    theme: &crate::ui::resolved_theme::ResolvedTheme,
    scrollbar: Option<&crate::ui::scrollbar::ScrollbarUiState>,
) {
    let mut list_items = Vec::new();
    let selected_job = jobs.get(ts.queue_cursor);
    let (sel_bg, sel_fg) = match selected_job.map(|j| &j.status) {
        Some(TransferJobStatus::Cancelled) => (theme.ui.danger, theme.ui.strong),
        Some(TransferJobStatus::Failed) => (theme.ui.error, theme.ui.strong),
        Some(TransferJobStatus::Paused) => (theme.ui.warning, theme.ui.badge_fg),
        Some(TransferJobStatus::Completed) => (theme.ui.success, theme.ui.badge_fg),
        _ => (theme.ui.accent, theme.ui.badge_fg),
    };

    for (idx, job) in jobs.iter().enumerate() {
        let is_selected = idx == ts.queue_cursor;

        let op_name = crate::ui::transfer::labels::operation(job.operation);
        let status_str = crate::ui::transfer::labels::status(job);
        let color = crate::ui::transfer::status_color(theme, job.status);

        let title_line = format!("#{} {} - {}", idx + 1, op_name, status_str);
        let dest_str = t("transfer_job_dest").replace(
            "{}",
            &job.destination
                .file_name()
                .unwrap_or(job.destination.as_os_str())
                .to_string_lossy(),
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
                    Style::default().fg(theme.ui.muted)
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
                .title(t("transfer_jobs_title"))
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.ui.muted)),
        )
        .highlight_style(Style::default().bg(sel_bg).fg(sel_fg));

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(Some(ts.queue_cursor));

    f.render_stateful_widget(jobs_list, area, &mut list_state);
    register_jobs(area, list_state.offset(), jobs.len(), ts.queue_cursor);

    // The bar counts jobs, as the list scrolls: JOB_ROWS rows each.
    let viewport = (area.height.saturating_sub(2) / JOB_ROWS).max(1) as usize;
    scrollbar::render_vertical_inside_block(
        f,
        area,
        ScrollView {
            content_len: jobs.len(),
            viewport_len: viewport,
            offset: list_state.offset(),
        },
        theme,
        ScrollTarget {
            surface: ScrollbarSurface::Popup,
            hits: scrollbar,
            id: ScrollTargetId::TransferJobs,
        },
    );
}

/// Rows of one job in the list: title, destination, blank.
const JOB_ROWS: u16 = 3;

/// Each job takes [`JOB_ROWS`] rows from the list's first shown job on; a
/// click on them moves the cursor there.
fn register_jobs(area: Rect, offset: usize, count: usize, cursor: usize) {
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    let mut y = inner.y;
    for index in offset..count {
        if y >= inner.bottom() {
            break;
        }
        let rows = Rect::new(inner.x, y, inner.width, JOB_ROWS).intersection(inner);
        crate::ui::popup::kit::mouse::list_item(rows, (index, cursor), &[]);
        y += JOB_ROWS;
    }
}
