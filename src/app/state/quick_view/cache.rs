//! Small bounded cache of quick-view previews keyed by file identity.

use super::load::QuickViewPreview;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// Number of previews kept in memory.
pub const QUICK_VIEW_CACHE_ENTRIES: usize = 32;

/// Identifies one version of a file: a changed mtime or size is a miss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewKey {
    pub path: PathBuf,
    pub modified: Option<SystemTime>,
    pub size: u64,
    pub allow_image: bool,
}

/// FIFO-evicting cache; hits are moved to the back (most recent).
#[derive(Debug)]
pub struct PreviewCache {
    capacity: usize,
    entries: VecDeque<(PreviewKey, Arc<QuickViewPreview>)>,
}

impl Default for PreviewCache {
    fn default() -> Self {
        Self::with_capacity(QUICK_VIEW_CACHE_ENTRIES)
    }
}

impl PreviewCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &PreviewKey) -> Option<Arc<QuickViewPreview>> {
        let idx = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(idx)?;
        let preview = Arc::clone(&entry.1);
        self.entries.push_back(entry);
        Some(preview)
    }

    pub fn insert(&mut self, key: PreviewKey, preview: Arc<QuickViewPreview>) {
        // Drop any stale version of the same file.
        self.entries.retain(|(k, _)| k.path != key.path);
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back((key, preview));
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
