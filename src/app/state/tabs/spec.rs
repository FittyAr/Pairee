//! Plain description of a tab (no UI or runtime types) for saving and
//! restoring sessions.

use crate::app::state::{PanelViewMode, SortField};
use crate::fs::vfs::PanelSource;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Kind of source a tab was showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Local,
    Archive,
    Remote,
}

impl SourceKind {
    pub fn of(source: &PanelSource) -> Self {
        match source {
            PanelSource::Local => Self::Local,
            PanelSource::Archive(_) => Self::Archive,
            PanelSource::Remote(_) => Self::Remote,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabSpec {
    pub path: PathBuf,
    pub source: SourceKind,
    pub view_mode: PanelViewMode,
    pub sort_field: SortField,
    pub sort_reverse: bool,
    pub show_long_names: bool,
    #[serde(default)]
    pub filter_mask: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub locked: bool,
}
