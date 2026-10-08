//! Review list: one row per difference with its action, a summary line,
//! and the delete confirmation on top.

use super::bytes;
use crate::app::state::popup::{SyncDialog, SyncReview};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::fs::sync::{SyncAction, SyncItem, SyncSummary};
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit::{self, ListPopup, Scroll, TextBox};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Text},
};

/// Width of the action and kind columns.
const ACTION_WIDTH: usize = 10;
const KIND_WIDTH: usize = 14;
const SIZE_WIDTH: usize = 10;

pub(super) fn render(
    f: &mut Frame,
    dialog: &SyncDialog,
    review: &SyncReview,
    theme: &Theme,
    size: Rect,
) {
    let summary = review.summary();
    let rows = review
        .visible()
        .into_iter()
        .map(|idx| row(&review.items[idx]))
        .collect();
    let header = vec![
        Line::from(summary_text(&summary)),
        Line::from(format!(
            " {:<ACTION_WIDTH$} {:<KIND_WIDTH$} {:>SIZE_WIDTH$}  {}",
            t("sync_col_action"),
            t("sync_col_status"),
            t("sync_col_size"),
            t("sync_col_path"),
        ))
        .style(Style::default().add_modifier(Modifier::BOLD)),
    ];
    ListPopup {
        area: centered_rect(90, 80, size),
        title: super::title(dialog.options.direction),
        border: Color::Yellow,
        empty: Some(t("sync_no_differences")),
        header,
        rows,
        cursor: review.cursor,
        scroll: Scroll::HalfPage,
        hint: Some(t("sync_review_hint")),
        scrollbar: None,
    }
    .render(f, theme);
    if review.confirming {
        render_confirm(f, &summary, theme, size);
    }
}

fn row(item: &SyncItem) -> (String, Style) {
    let color = match item.action {
        SyncAction::CopyToRight => Color::LightGreen,
        SyncAction::CopyToLeft => Color::LightCyan,
        SyncAction::DeleteLeft | SyncAction::DeleteRight => Color::LightRed,
        SyncAction::Skip => Color::DarkGray,
    };
    let size = item.left_bytes.max(item.right_bytes);
    let mut path = item.rel_path.display().to_string();
    if item.is_dir() {
        path.push(std::path::MAIN_SEPARATOR);
    }
    let text = format!(
        " {:<ACTION_WIDTH$} {:<KIND_WIDTH$} {:>SIZE_WIDTH$}  {}",
        t(item.action.label_key()),
        t(item.kind.label_key()),
        bytes(size),
        path,
    );
    (text, Style::default().fg(color))
}

fn summary_text(s: &SyncSummary) -> String {
    t("sync_summary")
        .replacen("{}", &s.copy_right.to_string(), 1)
        .replacen("{}", &bytes(s.copy_right_bytes), 1)
        .replacen("{}", &s.copy_left.to_string(), 1)
        .replacen("{}", &bytes(s.copy_left_bytes), 1)
        .replacen("{}", &s.delete.to_string(), 1)
        .replacen("{}", &bytes(s.delete_bytes), 1)
        .replacen("{}", &s.skipped.to_string(), 1)
        .replacen("{}", &s.equal.to_string(), 1)
}

fn render_confirm(f: &mut Frame, s: &SyncSummary, theme: &Theme, size: Rect) {
    let question = t("sync_confirm_delete")
        .replacen("{}", &s.delete.to_string(), 1)
        .replacen("{}", &bytes(s.delete_bytes), 1)
        .replacen("{}", &(s.copy_right + s.copy_left).to_string(), 1);
    TextBox {
        size: (60, 7),
        title: t("sync_confirm_title"),
        border: Style::default().fg(Color::LightRed),
        body: Text::from(vec![
            Line::from(""),
            Line::from(question),
            Line::from(""),
            Line::from(t("sync_confirm_hint")),
        ]),
        body_style: kit::popup_fg(theme),
    }
    .render_wrapped(f, size, theme);
}
