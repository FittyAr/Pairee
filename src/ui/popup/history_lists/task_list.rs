use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::scrollbar::ScrollbarUiState;
use crate::ui::theme_apply::parse_color;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub(super) fn render_task_list(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    match popup {
        PopupType::TaskListDialog {
            tasks,
            cursor_idx,
            filter_query,
            is_filtering,
        } => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(t("task_list_title"))
                .style(Style::default().bg(parse_color(&theme.popup_bg)));

            let inner = block.inner(area);
            f.render_widget(block, area);

            if tasks.is_empty() {
                let paragraph = Paragraph::new(t("task_list_empty"))
                    .style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            } else {
                let filter_active = *is_filtering || !filter_query.is_empty();
                let extra_lines_count = if filter_active { 2 } else { 0 };

                let list_height = inner.height.saturating_sub(4 + extra_lines_count) as usize;
                let scroll_start = cursor_idx.saturating_sub(list_height / 2);
                let mut lines = Vec::new();

                // Table header
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        " {:<8} | {:<35} | {:<12} ",
                        t("col_pid"),
                        t("col_process_name"),
                        t("col_memory")
                    ),
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )]));

                // If filter is active or query is not empty, show the filter input line
                if filter_active {
                    let prompt_text = if *is_filtering {
                        format!(" {}: {}_", t("task_list_filter"), filter_query)
                    } else {
                        format!(" {}: {}", t("task_list_filter"), filter_query)
                    };
                    lines.push(Line::from(Span::styled(
                        prompt_text,
                        Style::default().fg(Color::Yellow),
                    )));
                    lines.push(Line::from("")); // spacer
                }

                for (i, task) in tasks
                    .iter()
                    .enumerate()
                    .skip(scroll_start)
                    .take(list_height)
                {
                    let is_cursor = i == *cursor_idx;
                    let mem_mb = (task.memory_kb as f64) / 1024.0;
                    let line_str =
                        format!(" {:<8} | {:<35} | {:<12.1} ", task.pid, task.name, mem_mb);

                    let matches = filter_query.is_empty()
                        || task
                            .name
                            .to_lowercase()
                            .contains(&filter_query.to_lowercase());

                    let style = if is_cursor {
                        Style::default()
                            .bg(parse_color(&theme.selection_bg))
                            .fg(parse_color(&theme.selection_fg))
                            .add_modifier(Modifier::BOLD)
                    } else if !matches {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        Style::default().fg(parse_color(&theme.popup_fg))
                    };
                    lines.push(Line::from(Span::styled(line_str, style)));
                }

                let hint_key = if *is_filtering {
                    "task_list_hint_filtering"
                } else if !filter_query.is_empty() {
                    "task_list_hint_filtered"
                } else {
                    "task_list_hint_normal"
                };

                let hint = Line::from(Span::styled(
                    t(hint_key),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
                lines.push(hint);

                let paragraph =
                    Paragraph::new(lines).style(Style::default().fg(parse_color(&theme.popup_fg)));
                f.render_widget(paragraph, inner);
            }
            true
        }
        _ => false,
    }
}
