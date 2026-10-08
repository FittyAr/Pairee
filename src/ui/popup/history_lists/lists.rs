use crate::app::state::PopupType;
use crate::config::localization::t;
use crate::ui::popup::centered_rect;
use crate::ui::popup::kit::{self, ListPopup, Scroll};
use crate::ui::scrollbar::{ScrollTargetId, ScrollbarUiState};
use ratatui::{Frame, layout::Rect, style::Color};

/// Title, empty-list text and hint of each history list.
const COMMAND_KEYS: [&str; 3] = [
    "history_command_title",
    "history_command_empty",
    "history_command_hint",
];
const VIEW_KEYS: [&str; 3] = [
    "history_view_title",
    "history_view_empty",
    "history_view_hint",
];
const FOLDER_KEYS: [&str; 3] = [
    "history_folder_title",
    "history_folder_empty",
    "history_folder_hint",
];

/// Command, viewed-file and folder history lists.
pub(super) fn render_history_lists(
    f: &mut Frame,
    popup: &PopupType,
    theme: &crate::config::theme::Theme,
    size: Rect,
    scrollbar: Option<&ScrollbarUiState>,
) -> bool {
    let (width, keys, id, cursor, entries) = match popup {
        PopupType::CommandHistoryList {
            entries,
            cursor_idx,
        } => (
            60,
            COMMAND_KEYS,
            ScrollTargetId::HistoryCommand,
            *cursor_idx,
            entries.clone(),
        ),
        PopupType::FileViewHistoryList {
            entries,
            cursor_idx,
        } => (
            65,
            VIEW_KEYS,
            ScrollTargetId::HistoryView,
            *cursor_idx,
            lossy(entries),
        ),
        PopupType::FoldersHistoryList {
            entries,
            cursor_idx,
        } => (
            65,
            FOLDER_KEYS,
            ScrollTargetId::HistoryFolder,
            *cursor_idx,
            lossy(entries),
        ),
        _ => return false,
    };
    let [title, empty, hint] = keys;
    let style = kit::popup_fg(theme);
    ListPopup {
        area: centered_rect(width, 50, size),
        title: t(title),
        border: Color::Cyan,
        empty: Some(t(empty)),
        header: Vec::new(),
        rows: entries
            .into_iter()
            .map(|e| (format!(" {} ", e), style))
            .collect(),
        cursor,
        scroll: Scroll::Centered,
        hint: Some(t(hint)),
        scrollbar: Some((scrollbar, id)),
    }
    .render(f, theme);
    true
}

fn lossy(paths: &[std::path::PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}
