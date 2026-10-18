//! A native reader backed by the external 7-Zip (Decorator): 7z archives
//! whose codecs or filters the native reader lacks (BCJ2, PPMd, ARM64,
//! Deflate64...) are read by 7-Zip instead of failing.

use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::edit::{ArchiveEdit, OnAdded};
use super::format::{ArchiveReader, EntryMeta, Visitor};

pub struct Fallback {
    pub native: &'static dyn ArchiveReader,
    pub external: &'static dyn ArchiveReader,
}

/// The `native` answer, or the `external` one when native fails (the
/// native error is kept when both do: it names what was not understood).
fn either<T>(native: Result<T>, external: impl FnOnce() -> Result<T>) -> Result<T> {
    native.or_else(|error| external().map_err(|_| error))
}

/// One entry, whichever reader names it (`/` or `\` separators).
fn key(name: &str) -> PathBuf {
    super::safe_extract::ExtractGuard::sanitize(name).unwrap_or_else(|_| PathBuf::from(name))
}

impl ArchiveReader for Fallback {
    fn entries(&self, archive: &Path) -> Result<Vec<EntryMeta>> {
        either(self.native.entries(archive), || {
            self.external.entries(archive)
        })
    }

    fn visit(&self, archive: &Path, visit: &mut Visitor) -> Result<()> {
        self.visit_matching(archive, &|_| true, &|| false, visit)
    }

    /// When the native reader stops on an entry it cannot decode (a
    /// codec it lacks), 7-Zip reads the wanted entries not handed out yet.
    /// Failures of the visitor itself (and cancellations) end the reading.
    fn visit_matching(
        &self,
        archive: &Path,
        wanted: &dyn Fn(&str) -> bool,
        cancelled: &dyn Fn() -> bool,
        visit: &mut Visitor,
    ) -> Result<()> {
        let mut done: HashSet<PathBuf> = HashSet::new();
        let mut visitor_failed = false;
        let native = self
            .native
            .visit_matching(archive, wanted, cancelled, &mut |meta, data| {
                let next = visit(meta, data);
                match &next {
                    Ok(_) => {
                        done.insert(key(&meta.name));
                    }
                    Err(_) => visitor_failed = true,
                }
                next
            });
        match native {
            Err(error) if !visitor_failed && !cancelled() => either(Err(error), || {
                let rest = |name: &str| wanted(name) && !done.contains(&key(name));
                self.external
                    .visit_matching(archive, &rest, cancelled, visit)
            }),
            other => other,
        }
    }

    fn test(&self, archive: &Path) -> Result<()> {
        either(self.native.test(archive), || self.external.test(archive))
    }

    fn count_hint(&self, archive: &Path) -> usize {
        self.native.count_hint(archive)
    }

    fn writable(&self) -> bool {
        self.native.writable()
    }

    fn rewrite(&self, archive: &Path, edit: &ArchiveEdit, on_added: &mut OnAdded) -> Result<()> {
        self.native.rewrite(archive, edit, on_added)
    }
}
