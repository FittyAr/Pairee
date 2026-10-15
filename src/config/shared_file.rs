//! Files that several Pairee instances running at once write.
//!
//! Each instance keeps its own copy in memory and used to write it whole,
//! so the last one to save silently undid what the others had saved
//! (settings, history, frecency). The rule now is: **write only what this
//! instance changed, on top of what is on disk at that moment**.
//!
//! * Every read-merge-write runs under an advisory lock on a sibling
//!   `.<name>.lock` file ([`with_lock`]), so two instances cannot
//!   interleave. The data file itself is still replaced atomically
//!   (`write_atomic`), which is why the lock is a separate file.
//! * Lists and stores (history, frecency) are merged by their owners
//!   through [`merge_save`]; [`merge_recent`] merges newest-first lists.
//! * TOML documents (`config.toml`, `keybindings.toml`) remember what this
//!   instance last read or wrote ([`TomlBase`]): a save changes only the
//!   keys that differ from it and keeps everything else as found on disk.
//! * Files that are a snapshot of one instance (the session) are written
//!   under the lock too: the last writer wins, but whole.

use super::toml_store;
use crate::lock::LockExt;
use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a save waits for another instance to release the lock before
/// going ahead without it (a stopped process must not block saving). A
/// process that exits or crashes releases its lock at once, so this only
/// matters for a suspended one; it is generous because saves fsync, and on
/// a slow disk a few queued writers can take seconds.
const LOCK_WAIT: Duration = Duration::from_secs(10);
const LOCK_RETRY: Duration = Duration::from_millis(10);

/// The lock file of `path`: `.<name>.lock` next to it.
fn lock_path(path: &Path) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    path.with_file_name(format!(".{name}.lock"))
}

/// Holds the exclusive advisory lock of `path` (released when dropped);
/// `None` when the lock could not be taken in time or at all.
fn acquire(path: &Path) -> Option<File> {
    let lock = lock_path(path);
    if let Some(parent) = lock.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
    {
        Ok(file) => file,
        Err(e) => {
            log::warn!("Saving {} without a lock: {e}", path.display());
            return None;
        }
    };
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Some(file),
            Err(TryLockError::WouldBlock) if started.elapsed() < LOCK_WAIT => {
                std::thread::sleep(LOCK_RETRY);
            }
            Err(e) => {
                log::warn!("Saving {} without a lock: {e}", path.display());
                return None;
            }
        }
    }
}

/// Runs `f` while holding the advisory lock of `path`, so no other
/// instance reads or writes it meanwhile.
pub fn with_lock<R>(path: &Path, f: impl FnOnce() -> Result<R>) -> Result<R> {
    let _lock = acquire(path);
    f()
}

/// Under the lock: reads `path` (`None` when missing or unreadable), lets
/// `merge` combine it with this instance's data, and writes the result
/// atomically. Returns what was written.
pub fn merge_save<T: Serialize + DeserializeOwned>(
    path: &Path,
    merge: impl FnOnce(Option<T>) -> T,
) -> Result<T> {
    with_lock(path, || {
        let disk = match toml_store::read(path) {
            toml_store::Loaded::Ok(value) => Some(value),
            _ => None,
        };
        let merged = merge(disk);
        toml_store::save(path, &merged)?;
        Ok(merged)
    })
}

/// Merges a newest-first list (at most `cap` entries) this instance
/// changed from `base` to `mine` into `disk`, as another instance may have
/// saved it meanwhile: the entries this instance put in front come first,
/// then the disk's, without the ones this instance removed.
pub fn merge_recent<T: PartialEq + Clone>(
    base: &[T],
    mine: &[T],
    disk: &[T],
    cap: usize,
) -> Vec<T> {
    // `mine` is its pushed entries followed by what is left of `base`, in
    // `base`'s order: find where the pushed part ends.
    let pushed = (0..=mine.len())
        .find(|&split| {
            let (front, rest) = mine.split_at(split);
            let kept = base
                .iter()
                .filter(|b| !front.contains(b) && rest.contains(b));
            rest.iter().eq(kept)
        })
        .unwrap_or(mine.len());
    let front = &mine[..pushed];
    let removed = |item: &T| base.contains(item) && !mine.contains(item);
    let mut merged: Vec<T> = front.to_vec();
    merged.extend(
        disk.iter()
            .filter(|d| !removed(d) && !front.contains(d))
            .cloned(),
    );
    merged.truncate(cap);
    merged
}

/// Three-way merge of TOML values: what this instance changed from `base`
/// to `mine` applied to `disk`. Tables merge key by key; any other value
/// this instance changed (or removed) replaces the disk's whole.
pub fn merge_toml(base: &toml::Value, mine: &toml::Value, disk: &toml::Value) -> toml::Value {
    merge_value(Some(base), Some(mine), Some(disk)).unwrap_or_else(|| mine.clone())
}

fn merge_value(
    base: Option<&toml::Value>,
    mine: Option<&toml::Value>,
    disk: Option<&toml::Value>,
) -> Option<toml::Value> {
    if mine == base {
        return disk.cloned();
    }
    let (Some(toml::Value::Table(mine)), Some(toml::Value::Table(disk))) = (mine, disk) else {
        return mine.cloned();
    };
    let base = match base {
        Some(toml::Value::Table(base)) => Some(base),
        _ => None,
    };
    let keys = mine
        .keys()
        .chain(disk.keys().filter(|k| !mine.contains_key(*k)));
    let merged = keys
        .filter_map(|key| {
            let value = merge_value(base.and_then(|b| b.get(key)), mine.get(key), disk.get(key))?;
            Some((key.clone(), value))
        })
        .collect();
    Some(toml::Value::Table(merged))
}

/// What this instance last read or wrote of a TOML document, so that a
/// save keeps the keys other instances changed meanwhile (see
/// [`merge_toml`]). Without it (nothing remembered yet) a save writes the
/// whole document.
#[derive(Debug, Default)]
pub struct TomlBase(Mutex<Option<toml::Value>>);

impl Clone for TomlBase {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock_safe().clone()))
    }
}

/// `text` as a TOML document, if it is one.
fn parse(text: &str) -> Option<toml::Value> {
    toml::from_str::<toml::Table>(text)
        .ok()
        .map(toml::Value::Table)
}

impl TomlBase {
    /// Records `text` (this instance's document, as it would write it) as
    /// what it holds now.
    pub fn remember(&self, text: &str) {
        *self.0.lock_safe() = parse(text);
    }

    /// Writes `text`, this instance's whole document, to `path`. When the
    /// file changed on disk since this instance last read or wrote it, only
    /// the keys this instance changed are written over it.
    pub fn save(&self, path: &Path, text: &str) -> Result<()> {
        let mine = parse(text);
        with_lock(path, || {
            let mut base = self.0.lock_safe();
            let disk_text = std::fs::read_to_string(path).ok();
            let disk = disk_text.as_deref().and_then(parse);
            if disk_text.is_some() && disk.is_none() {
                // Hand-edited into something unreadable: keep it before
                // the save replaces it.
                toml_store::set_aside(path, "not a valid TOML document");
            }
            let merged = match (base.as_ref(), mine.as_ref(), disk) {
                (Some(base), Some(mine), Some(disk)) if disk != *base => {
                    Some(toml::to_string_pretty(&merge_toml(base, mine, &disk))?)
                }
                _ => None,
            };
            let out = merged.as_deref().unwrap_or(text);
            super::write_atomic(path, out.as_bytes())?;
            *base = mine;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
