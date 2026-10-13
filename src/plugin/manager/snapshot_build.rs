//! Builds the [`AppStateSnapshot`] a plugin reads as `pairee.cx` (and the
//! legacy `pairee.app` snapshot) from the application state.

use super::snapshot::{
    AppStateSnapshot, FileEntrySnapshot, TabSnapshot, TaskSnapshot, YankedSnapshot,
};
use crate::app::state::file_clipboard::ClipMode;
use crate::app::state::{ActivePanel, AppState};
use crate::fs::vfs::PanelSource;
use std::path::Path;

/// `path` on `source` as a URL: `scheme://user@host:port/path` on a remote
/// connection, the plain path otherwise.
fn url_on(source: &PanelSource, path: &Path) -> String {
    match source {
        PanelSource::Remote(conn) => conn.info().url_of(path),
        _ => path.to_string_lossy().into_owned(),
    }
}

/// A file of a panel on `source`, with its URL.
fn file_on(source: &PanelSource, entry: &crate::fs::FileEntry) -> FileEntrySnapshot {
    let mut file = FileEntrySnapshot::from_file_entry(entry);
    file.url = url_on(source, &entry.path);
    file
}

/// Read-only view of the panels, tabs, transfers and file clipboard.
pub fn build_snapshot(state: &AppState) -> AppStateSnapshot {
    let active = state.get_active_panel();
    let hovered = active
        .entries
        .get(active.cursor_index)
        .map(|e| file_on(&active.source, e));
    let selected = active
        .entries
        .iter()
        .filter(|e| active.selection_order.contains(&e.path))
        .map(|e| file_on(&active.source, e))
        .collect();
    let panel = |side| state.panels.side(side);
    let cwd = |side| panel(side).current_path.to_string_lossy().to_string();
    let url = |side: ActivePanel| url_on(&panel(side).source, &panel(side).current_path);
    let tabs = state.panels.tabs(state.panels.active);
    AppStateSnapshot {
        active_panel: format!("{:?}", state.panels.active).to_lowercase(),
        left_cwd: cwd(ActivePanel::Left),
        right_cwd: cwd(ActivePanel::Right),
        hovered_file: hovered,
        selected_files: selected,
        left_url: url(ActivePanel::Left),
        right_url: url(ActivePanel::Right),
        tabs: tabs
            .tabs()
            .iter()
            .map(|tab| TabSnapshot {
                url: url_on(&tab.panel.source, &tab.panel.current_path),
                name: tab.name.clone(),
                locked: tab.lock.is_some(),
            })
            .collect(),
        active_tab: tabs.active_index(),
        tasks: tasks(state),
        yanked: state.file_clipboard.as_ref().map(|clip| YankedSnapshot {
            cut: clip.mode == ClipMode::Cut,
            urls: clip.paths.iter().map(|p| url_on(&clip.source, p)).collect(),
        }),
    }
}

/// The transfer engine's jobs, oldest first.
fn tasks(state: &AppState) -> Vec<TaskSnapshot> {
    let Some(transfer) = &state.transfer else {
        return Vec::new();
    };
    transfer
        .engine
        .queue
        .get_all()
        .iter()
        .map(|job| {
            let progress = job.progress.as_ref();
            TaskSnapshot {
                id: job.id.to_string(),
                kind: format!("{:?}", job.operation).to_lowercase(),
                status: format!("{:?}", job.status).to_lowercase(),
                sources: job.sources.len(),
                destination: job.destination.to_string_lossy().into_owned(),
                files_total: progress.map_or(0, |p| p.files_total),
                files_done: progress.map_or(0, |p| p.files_completed),
                bytes_total: progress.map_or(0, |p| p.bytes_total),
                bytes_done: progress.map_or(0, |p| p.bytes_transferred),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::file_clipboard::FileClipboard;
    use std::path::PathBuf;

    #[test]
    fn snapshot_has_tabs_clipboard_and_urls() {
        let mut state = AppState::new(PathBuf::from("/l"), PathBuf::from("/r"));
        state.file_clipboard = Some(FileClipboard {
            paths: vec![PathBuf::from("/l/a"), PathBuf::from("/l/b")],
            mode: ClipMode::Cut,
            source: PanelSource::Local,
        });

        let snapshot = build_snapshot(&state);

        assert_eq!(snapshot.left_url, snapshot.left_cwd);
        assert_eq!(snapshot.tabs.len(), 1);
        assert_eq!(snapshot.active_tab, 0);
        assert!(snapshot.tasks.is_empty());
        let yanked = snapshot.yanked.unwrap();
        assert!(yanked.cut);
        assert_eq!(yanked.urls.len(), 2);
    }
}
