//! Rebuilds the panels from a saved [`SessionFile`] and applies the start
//! folders given on the command line.

use crate::app::state::tabs::spec::{SourceKind, TabSpec};
use crate::app::state::tabs::{PanelTabs, PendingRemote, Tab};
use crate::app::state::{ActivePanel, AppState};
use crate::config::localization::t;
use crate::config::session::SessionFile;
use crate::config::settings::Settings;
use std::path::{Path, PathBuf};

/// The user's home folder (the current folder, then the filesystem root,
/// when it cannot be determined).
pub fn home_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

/// The folder itself, else its nearest existing ancestor, else home.
pub fn nearest_existing_dir(path: &Path) -> PathBuf {
    path.ancestors()
        .find(|p| !p.as_os_str().is_empty() && p.is_dir())
        .map(Path::to_path_buf)
        .unwrap_or_else(home_dir)
}

/// Replaces both sides with the saved tabs and layout. Returns the notices
/// to show (SSH tabs that cannot be reopened).
pub fn restore(state: &mut AppState, session: &SessionFile, settings: &Settings) -> Vec<String> {
    let mut notices = Vec::new();
    for side in [ActivePanel::Left, ActivePanel::Right] {
        let saved = session.side(side);
        let tabs = saved
            .tabs
            .iter()
            .map(|spec| restore_tab(spec, settings, &mut notices))
            .collect();
        if let Some(tabs) = PanelTabs::from_tabs(tabs, saved.active) {
            *state.panels.tabs_mut(side) = tabs;
        }
    }
    let panels = &mut state.panels;
    panels.active = session.active_side;
    panels.left_visible = session.left_visible;
    panels.right_visible = session.right_visible;
    panels.quick_view_active = session.quick_view;
    notices
}

/// One saved tab. Missing folders fall back to their nearest existing
/// ancestor; SSH tabs wait (at home) until they are shown.
fn restore_tab(spec: &TabSpec, settings: &Settings, notices: &mut Vec<String>) -> Tab {
    let mut spec = spec.clone();
    let remote = match spec.source {
        SourceKind::Remote => remote_target(&spec, settings, notices),
        SourceKind::Archive if archive_exists(&spec.path) => None,
        SourceKind::Archive | SourceKind::Local => {
            let folder = nearest_existing_dir(&spec.path);
            if folder != spec.path {
                log::info!(
                    "Session folder {} is gone; opening {}",
                    spec.path.display(),
                    folder.display()
                );
                spec.cursor = None;
            }
            spec.path = folder;
            spec.source = SourceKind::Local;
            None
        }
    };
    if spec.source == SourceKind::Remote {
        spec.path = home_dir();
        spec.cursor = None;
    }
    let mut tab = Tab::from_spec(&spec);
    tab.pending_remote = remote;
    tab
}

/// The connection to reopen for a saved SSH tab, or `None` (with a notice)
/// when its preset is gone or SSH is turned off.
fn remote_target(
    spec: &TabSpec,
    settings: &Settings,
    notices: &mut Vec<String>,
) -> Option<PendingRemote> {
    let preset = spec.ssh_preset.as_deref().unwrap_or_default();
    let known = settings.ssh_presets.iter().any(|p| p.name == preset);
    if settings.ssh_enabled && known {
        return Some(PendingRemote {
            preset: preset.to_string(),
            path: spec.path.clone(),
            cursor: spec.cursor.clone(),
            connecting: false,
        });
    }
    notices.push(t("session_ssh_preset_missing").replace("{}", preset));
    None
}

/// An archive tab can be reopened while the archive file itself exists.
fn archive_exists(path: &Path) -> bool {
    path.ancestors().any(Path::is_file)
}

/// Points the shown tab of the left (then right) side at the folders given
/// on the command line. A file opens its folder with the cursor on it.
pub fn apply_start_paths(state: &mut AppState, paths: &[PathBuf]) {
    for (side, path) in [ActivePanel::Left, ActivePanel::Right]
        .into_iter()
        .zip(paths)
    {
        let tab = state.panels.tabs_mut(side).active_mut();
        let mut spec = tab.spec();
        let (folder, cursor) = start_target(path);
        spec.path = folder;
        spec.cursor = cursor;
        spec.source = SourceKind::Local;
        spec.ssh_preset = None;
        spec.locked = false;
        *tab = Tab::from_spec(&spec);
    }
}

/// Folder to open for a command-line path, and the entry to focus in it.
fn start_target(path: &Path) -> (PathBuf, Option<String>) {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    if absolute.is_file()
        && let Some(parent) = absolute.parent()
    {
        let name = absolute
            .file_name()
            .map(|n| n.to_string_lossy().into_owned());
        return (parent.to_path_buf(), name);
    }
    (nearest_existing_dir(&absolute), None)
}
