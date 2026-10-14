use crate::app::state::popup::{CommandHistoryList, FileViewHistoryList, FoldersHistoryList};
use crate::config::localization::t;
use crate::ui::popup::centered_rect_in;
use crate::ui::popup::kit::{ListPopup, Scroll};
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::scrollbar::ScrollTargetId;
use ratatui::Frame;

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

/// What tells the three history lists apart.
struct ListKind {
    width: u16,
    keys: [&'static str; 3],
    id: ScrollTargetId,
}

/// A history list of `entries` with the cursor on `cursor`.
fn history_list(
    f: &mut Frame,
    cx: &ViewCtx<'_>,
    kind: ListKind,
    cursor: usize,
    entries: Vec<String>,
) {
    let [title, empty, hint] = kind.keys;
    let style = cx.theme.popup_fg();
    ListPopup {
        area: centered_rect_in(kind.width, 50, cx.area),
        title: t(title),
        border: cx.theme.ui.accent,
        empty: Some(t(empty)),
        header: Vec::new(),
        rows: entries
            .into_iter()
            .map(|e| (format!(" {} ", e), style))
            .collect(),
        cursor,
        scroll: Scroll::Centered,
        hint: Some(t(hint)),
        scrollbar: Some((cx.scrollbar(), kind.id)),
    }
    .render(f, cx.theme);
}

impl DialogView for CommandHistoryList {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let kind = ListKind {
            width: 60,
            keys: COMMAND_KEYS,
            id: ScrollTargetId::HistoryCommand,
        };
        history_list(f, cx, kind, self.cursor_idx, self.entries.clone());
    }
}

impl DialogView for FileViewHistoryList {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let kind = ListKind {
            width: 65,
            keys: VIEW_KEYS,
            id: ScrollTargetId::HistoryView,
        };
        history_list(f, cx, kind, self.cursor_idx, lossy(&self.entries));
    }
}

impl DialogView for FoldersHistoryList {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let kind = ListKind {
            width: 65,
            keys: FOLDER_KEYS,
            id: ScrollTargetId::HistoryFolder,
        };
        history_list(f, cx, kind, self.cursor_idx, lossy(&self.entries));
    }
}

fn lossy(paths: &[std::path::PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}
