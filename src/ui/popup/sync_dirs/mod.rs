//! Synchronize folders dialog (options form, review list, delete
//! confirmation) and the folder-scan progress popup.

mod options;
mod review;
#[cfg(test)]
mod tests;

use crate::app::state::AppState;
use crate::app::state::popup::{FolderScanProgress, SyncDialog};
use crate::config::localization::t;
use crate::fs::sync::SyncDirection;
use crate::ui::popup::kit::TextBox;
use crate::ui::popup::view::{DialogView, ViewCtx};
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
};

impl DialogView for SyncDialog {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        match &self.review {
            Some(review) => review::render(f, self, review, cx.theme, cx.area),
            None => options::render(f, self, cx.theme, cx.area),
        }
    }
}

impl DialogView for FolderScanProgress {
    fn render(&self, f: &mut Frame, cx: &ViewCtx<'_>) {
        render_progress(f, cx.state, cx.theme, cx.area);
    }
}

/// `" Synchronize folders: Left → Right "`.
fn title(direction: SyncDirection) -> String {
    t("sync_title_with_direction").replacen("{}", &t(direction.label_key()), 1)
}

fn bytes(n: u64) -> String {
    bytesize::ByteSize::b(n).to_string()
}

fn render_progress(f: &mut Frame, state: &AppState, theme: &ResolvedTheme, size: Rect) {
    let title_key = state
        .folder_scan
        .purpose()
        .map_or("compare_scan_title", |p| p.title_key());
    let progress = state.folder_scan.progress().unwrap_or_default();
    let counts = t("sync_scan_progress")
        .replacen("{}", &progress.dirs.to_string(), 1)
        .replacen("{}", &progress.files.to_string(), 1);
    let body = Text::from(vec![
        counts.into(),
        progress.current.display().to_string().into(),
        "".into(),
        t("sync_scan_hint").into(),
    ]);
    TextBox {
        size: (64, 6),
        title: t(title_key),
        border: Style::default().fg(Color::Cyan),
        body,
        body_style: theme.popup_fg(),
    }
    .render(f, size, theme);
}
