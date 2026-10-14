//! A short-lived memory of the last folder listing `stat` read.
//!
//! Without `MLST`, `stat` lists the parent folder. A copy asks about every
//! file of a folder in turn, so reading the whole folder per file is
//! quadratic; here the listing is kept for a moment and every change made
//! through the adapter (including the end of an upload) drops it.

use crate::fs::vfs::VfsEntry;
use crate::lock::LockExt;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a listing answers `stat` calls.
const TTL: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct StatCache {
    slot: Mutex<Option<Cached>>,
}

struct Cached {
    dir: String,
    at: Instant,
    entries: Vec<VfsEntry>,
}

impl StatCache {
    /// The entry `name` of the folder `dir`, from the cached listing or from
    /// `load` (whose result is then cached).
    pub fn find(
        &self,
        dir: &str,
        name: &str,
        load: impl FnOnce() -> io::Result<Vec<VfsEntry>>,
    ) -> io::Result<Option<VfsEntry>> {
        let mut slot = self.slot.lock_safe();
        let fresh = slot
            .as_ref()
            .is_some_and(|c| c.dir == dir && c.at.elapsed() < TTL);
        if !fresh {
            *slot = None;
            let entries = load()?;
            *slot = Some(Cached {
                dir: dir.to_string(),
                at: Instant::now(),
                entries,
            });
        }
        let cached = slot.as_ref().expect("filled above");
        Ok(cached.entries.iter().find(|e| e.name == name).cloned())
    }

    /// Forgets the listing: something changed.
    pub fn invalidate(&self) {
        *self.slot.lock_safe() = None;
    }
}

/// A writer that drops the cached listing once it is flushed or dropped
/// (the file it uploads then exists, or has a new size).
pub struct Invalidating {
    inner: Box<dyn Write + Send>,
    cache: Arc<StatCache>,
}

impl Invalidating {
    pub fn new(inner: Box<dyn Write + Send>, cache: Arc<StatCache>) -> Self {
        cache.invalidate();
        Self { inner, cache }
    }
}

impl Write for Invalidating {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = self.inner.flush();
        self.cache.invalidate();
        result
    }
}

impl Drop for Invalidating {
    fn drop(&mut self) {
        self.cache.invalidate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn entry(name: &str) -> VfsEntry {
        VfsEntry {
            name: name.into(),
            path: PathBuf::from(name),
            ..VfsEntry::default()
        }
    }

    #[test]
    fn one_listing_answers_many_lookups_until_invalidated() {
        let cache = StatCache::default();
        let mut loads = 0;
        for name in ["a", "b", "missing"] {
            let found = cache
                .find("/d", name, || {
                    loads += 1;
                    Ok(vec![entry("a"), entry("b")])
                })
                .unwrap();
            assert_eq!(found.is_some(), name != "missing");
        }
        assert_eq!(loads, 1);
        cache.invalidate();
        cache
            .find("/d", "a", || {
                loads += 1;
                Ok(vec![])
            })
            .unwrap();
        assert_eq!(loads, 2);
        // Another folder is another listing.
        cache
            .find("/e", "a", || {
                loads += 1;
                Ok(vec![])
            })
            .unwrap();
        assert_eq!(loads, 3);
    }

    #[test]
    fn a_finished_upload_drops_the_listing() {
        let cache = Arc::new(StatCache::default());
        cache.find("/d", "new", || Ok(vec![entry("a")])).unwrap();
        let mut writer = Invalidating::new(Box::new(Vec::new()), Arc::clone(&cache));
        writer.flush().unwrap();
        let mut reloaded = false;
        cache
            .find("/d", "new", || {
                reloaded = true;
                Ok(vec![entry("new")])
            })
            .unwrap();
        assert!(reloaded);
    }
}
