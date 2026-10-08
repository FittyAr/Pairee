//! Synchronize folders dialog (options form, review list, delete
//! confirmation) and the folder-scan progress popup.

mod options;
mod review;
#[cfg(test)]
mod tests;

use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::config::theme::Theme;
use crate::fs::sync::SyncDirection;
use crate::ui::popup::kit::{self, TextBox};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Text,
};

pub fn render(
    f: &mut Frame,
    popup: &PopupType,
    state: &AppState,
    theme: &Theme,
    size: Rect,
) -> bool {
    match popup {
        PopupType::SyncDirs(dialog) => {
            match &dialog.review {
                Some(review) => review::render(f, dialog, review, theme, size),
                None => options::render(f, dialog, theme, size),
            }
            true
        }
        PopupType::FolderScanProgress => {
            render_progress(f, state, theme, size);
            true
        }
        _ => false,
    }
}

/// `" Synchronize folders: Left → Right "`.
fn title(direction: SyncDirection) -> String {
    t("sync_title_with_direction").replacen("{}", &t(direction.label_key()), 1)
}

fn bytes(n: u64) -> String {
    bytesize::ByteSize::b(n).to_string()
}

fn render_progress(f: &mut Frame, state: &AppState, theme: &Theme, size: Rect) {
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
        body_style: kit::popup_fg(theme),
    }
    .render(f, size, theme);
}
