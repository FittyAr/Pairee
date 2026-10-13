use crate::app::state::popup::TaskListDialog;
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
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
                " {:<8} | {:<35} | {:<12} ",
                t("col_pid"),
                t("col_process_name"),
                t("col_memory")
            ),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ))];
        if *is_filtering || !filter_query.is_empty() {
            let typing = if *is_filtering { "_" } else { "" };
            header.push(Line::from(Span::styled(
                format!(" {}: {}{}", t("task_list_filter"), filter_query, typing),
                Style::default().fg(Color::Yellow),
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
                    Style::default().fg(Color::DarkGray)
                };
                let mem_mb = (task.memory_kb as f64) / 1024.0;
                (
                    format!(" {:<8} | {:<35} | {:<12.1} ", task.pid, task.name, mem_mb),
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
            border: Color::Cyan,
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
