use crate::app::state::popup::TaskListDialog;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::text_width::pad_to_width;
use ratatui::{
    Frame,
    style::{Modifier, Style},
    text::{Line, Span},
};

impl DialogView for TaskListDialog {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let (theme, size) = (cx.theme, cx.area);
        let TaskListDialog {
            tasks,
            cursor_idx,
            filter_query,
            is_filtering,
        } = self;
        let mut header = vec![Line::from(Span::styled(
            format!(
                " {} | {} | {} ",
                pad_to_width(&t("col_pid"), 8),
                pad_to_width(&t("col_process_name"), 35),
                pad_to_width(&t("col_memory"), 12)
            ),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ))];
        if *is_filtering || !filter_query.is_empty() {
            let typing = if *is_filtering { "_" } else { "" };
            header.push(Line::from(Span::styled(
                format!(" {}: {}{}", t("task_list_filter"), filter_query, typing),
                Style::default().fg(theme.ui.warning),
            )));
            header.push(Line::from(""));
        }
        let filter = crate::app::text_filter::TextFilter::new(filter_query.text());
        let rows = tasks
            .iter()
            .map(|task| {
                let matches = filter.matches(&task.name);
                let style = if matches {
                    theme.popup_fg()
                } else {
                    Style::default().fg(theme.ui.muted)
                };
                let mem_mb = (task.memory_kb as f64) / 1024.0;
                (
                    format!(
                        " {:<8} | {} | {:<12.1} ",
                        task.pid,
                        pad_to_width(&task.name, 35),
                        mem_mb
                    ),
                    style,
                )
            })
            .collect();
        let hint_key = if *is_filtering {
            "task_list_hint_filtering"
        } else if !filter_query.is_empty() {
            "task_list_hint_filtered"
        } else {
            "task_list_hint_normal"
        };
        ListPopup {
            area: centered_rect_in(70, 60, size),
            title: t("task_list_title"),
            border: theme.ui.accent,
            empty: Some(t("task_list_empty")),
            header,
            rows,
            cursor: *cursor_idx,
            scroll: Scroll::HalfPage,
            hint: Some(t(hint_key)),
            scrollbar: None,
        }
        .render(f, theme);
    }
}
