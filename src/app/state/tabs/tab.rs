//! One folder tab: a panel location with its own listing, cursor,
//! selection, view and background jobs.

use super::spec::{SourceKind, TabSpec};
use crate::app::state::PanelState;
use crate::fs::vfs::PanelSource;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Stable identity of a tab, unique across both sides for the whole run.
/// Background results are routed by it, never by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabId(u64);

impl TabId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// Where a locked tab stays: navigating away opens a new tab instead.
#[derive(Debug, Clone)]
pub struct TabLock {
    pub path: PathBuf,
    pub source: PanelSource,
}

/// A restored SSH tab waiting for its connection, opened the first time
/// the tab is shown (see `app::session::remote`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRemote {
    /// Name of the SSH preset to connect with.
    pub preset: String,
    /// Remote folder to show once connected.
    pub path: PathBuf,
    /// Entry to put the cursor on once the remote folder is listed.
    pub cursor: Option<String>,
    /// The connection was started (it is not started twice).
    pub connecting: bool,
}

#[derive(Debug)]
pub struct Tab {
    pub id: TabId,
    pub panel: PanelState,
    /// Title chosen by the user (`None`: the folder name).
    pub name: Option<String>,
    pub lock: Option<TabLock>,
    pub pending_remote: Option<PendingRemote>,
}

impl Tab {
    pub fn new(panel: PanelState) -> Self {
        Self {
            id: TabId::next(),
            panel,
            name: None,
            lock: None,
            pending_remote: None,
        }
    }

    /// A tab restored from its plain description. Remote sources cannot
    /// be restored (they need a new connection) and start local; archive
    /// sources are reopened from the path by the first listing.
    pub fn from_spec(spec: &TabSpec) -> Self {
        let mut panel = PanelState::new(spec.path.clone());
        panel.view_mode = spec.view_mode;
        panel.sort_field = spec.sort_field;
        panel.sort_reverse = spec.sort_reverse;
        panel.show_long_names = spec.show_long_names;
        panel.filter_mask = spec.filter_mask.clone();
        panel.pending_focus = spec.cursor.clone();
        let mut tab = Self::new(panel);
        tab.name = spec.name.clone();
        if spec.locked {
            tab.lock_here();
        }
        tab
    }

    /// The plain, serializable description of this tab. A restored SSH tab
    /// that was never shown keeps its remote folder and preset.
    pub fn spec(&self) -> TabSpec {
        let panel = &self.panel;
        let (path, source, ssh_preset, cursor) = match &self.pending_remote {
            Some(remote) => (
                remote.path.clone(),
                SourceKind::Remote,
                Some(remote.preset.clone()),
                remote.cursor.clone(),
            ),
            None => (
                panel.current_path.clone(),
                SourceKind::of(&panel.source),
                None,
                panel
                    .entries
                    .get(panel.cursor_index)
                    .map(|entry| entry.name.clone())
                    .or_else(|| panel.pending_focus.clone()),
            ),
        };
        TabSpec {
            path,
            source,
            view_mode: panel.view_mode,
            sort_field: panel.sort_field,
            sort_reverse: panel.sort_reverse,
            show_long_names: panel.show_long_names,
            filter_mask: panel.filter_mask.clone(),
            name: self.name.clone(),
            locked: self.lock.is_some(),
            cursor,
            ssh_preset,
        }
    }

    /// A new, unlocked tab on the same folder and source, showing the
    /// current listing until its own reread arrives.
    pub fn duplicate(&self) -> Self {
        let mut tab = Self::from_spec(&self.spec());
        tab.name = None;
        tab.lock = None;
        tab.pending_remote = None;
        let (from, to) = (&self.panel, &mut tab.panel);
        to.current_path = from.current_path.clone();
        to.source = from.source.clone();
        to.entries = from.entries.clone();
        to.cursor_index = from.cursor_index;
        to.listed_path = from.listed_path.clone();
        to.attrs = from.attrs.clone();
        to.git_branch = from.git_branch.clone();
        to.git_statuses = from.git_statuses.clone();
        to.free_space = from.free_space;
        tab
    }

    /// Pins the tab to its current folder.
    pub fn lock_here(&mut self) {
        self.lock = Some(TabLock {
            path: self.panel.current_path.clone(),
            source: self.panel.source.clone(),
        });
    }

    pub fn toggle_lock(&mut self) {
        if self.lock.take().is_none() {
            self.lock_here();
        }
    }

    /// True when the tab is locked and was pointed somewhere else.
    pub fn left_its_lock(&self) -> bool {
        self.lock
            .as_ref()
            .is_some_and(|lock| lock.path != self.panel.current_path)
    }

    /// Display title: the user's name, else the folder (or archive) name,
    /// prefixed with the host on SFTP panels.
    pub fn title(&self) -> String {
        if let Some(name) = &self.name {
            return name.clone();
        }
        let path = &self.panel.current_path;
        let folder = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());
        match self.panel.source.ssh() {
            Some(client) => format!("{}:{folder}", client.info().host),
            None => folder,
        }
    }
}
