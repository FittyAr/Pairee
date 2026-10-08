use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit::{self, ListPopup, Scroll};
use crate::ui::scrollbar::ScrollbarUiState;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

pub(super) fn render_task_list(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    _scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    let PopupType::TaskListDialog {
        tasks,
        cursor_idx,
        filter_query,
        is_filtering,
    } = popup
    else {
        return false;
    };
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
    let needle = filter_query.to_lowercase();
    let rows = tasks
        .iter()
        .map(|task| {
            let matches = needle.is_empty() || task.name.to_lowercase().contains(&needle);
            let style = if matches {
                kit::popup_fg(theme)
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
        area: centered_rect(70, 60, size),
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
    true
}
