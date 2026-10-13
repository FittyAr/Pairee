//! Folder jump list: the query and the matching visited folders.

use crate::app::state::popup::FolderJump;
use crate::config::localization::t;
use crate::ui::popup::kit::FilterListView;
use crate::ui::popup::view::{DialogView, ViewCtx};
use ratatui::Frame;

impl DialogView for FolderJump {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        let empty = self.items.is_empty().then(|| t("folder_jump_empty"));
        FilterListView {
            max_width: 90,
            title: t("folder_jump_title"),
            query: &self.query,
            rows: self
                .items
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            cursor: self.cursor_idx,
            hint: empty.or_else(|| Some(t("folder_jump_hint"))),
        }
        .render(f, cx.area, cx.theme);
    }
}
