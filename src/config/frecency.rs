//! Frecency of the visited folders (as zoxide ranks them): every visit
//! raises a folder's rank, and the score weighs the rank by how recently it
//! was visited. The folder jump dialog and the `z` command line pick from
//! it.
//!
//! The store stays small: once the ranks add up to [`MAX_TOTAL_RANK`] they
//! all shrink (folders rarely visited fall out), and at most
//! [`MAX_FOLDERS`] are kept. It is written atomically next to the
//! configuration ([`super::toml_store`]), only when the folders history is
//! saved.

use anyhow::Result;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "frecency.toml";
/// Most folders kept.
pub const MAX_FOLDERS: usize = 500;
/// Sum of the ranks above which they all shrink (zoxide's `_ZO_MAXAGE`).
pub const MAX_TOTAL_RANK: f64 = 10_000.0;
/// Ranks are scaled to this share of [`MAX_TOTAL_RANK`] when aged.
const AGED_SHARE: f64 = 0.9;

const HOUR: u64 = 60 * 60;
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;

/// Visits of one folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FolderVisits {
    pub path: PathBuf,
    /// Grows by one per visit; shrinks when the store ages.
    pub rank: f64,
    /// Seconds since the Unix epoch.
    pub last_visit: u64,
}

impl FolderVisits {
    /// Rank weighted by recency: ×4 within the hour, ×2 within the day,
    /// ×½ within the week, ×¼ after that.
    pub fn score(&self, now: u64) -> f64 {
        let age = now.saturating_sub(self.last_visit);
        let weight = if age < HOUR {
            4.0
        } else if age < DAY {
            2.0
        } else if age < WEEK {
            0.5
        } else {
            0.25
        };
        self.rank * weight
    }
}

/// A folder matching a query, best first (see [`FrecencyStore::ranked`]).
#[derive(Debug, Clone, PartialEq)]
pub struct FolderMatch {
    pub path: PathBuf,
    pub score: f64,
    /// The last word of the query matches the folder's own name.
    pub name_hit: bool,
}

/// Every folder with its visits.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FrecencyStore {
    #[serde(default)]
    pub folders: Vec<FolderVisits>,
    /// The folders as last loaded or saved: a save adds only the visits
    /// made since, so other running instances' visits are kept.
    #[serde(skip)]
    saved: Vec<FolderVisits>,
}

impl FrecencyStore {
    /// The saved store (empty when missing or unreadable).
    pub fn load() -> Self {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Self {
        let mut store: Self = super::toml_store::load_or_default(path);
        store.saved = store.folders.clone();
        store
    }

    pub fn save(&mut self) -> Result<()> {
        self.save_to(&default_path())
    }

    /// Writes the visits made since the last load or save on top of the
    /// store as it is on disk now (other instances may have saved theirs).
    pub fn save_to(&mut self, path: &Path) -> Result<()> {
        super::shared_file::merge_save(path, |disk: Option<Self>| {
            self.merged(disk.unwrap_or_default(), now())
        })?;
        self.saved = self.folders.clone();
        Ok(())
    }

    /// Empties the saved store (the folders history is not kept).
    pub fn forget_saved() -> Result<()> {
        let path = default_path();
        super::shared_file::with_lock(&path, || super::toml_store::save(&path, &Self::default()))
    }

    /// `disk` plus this instance's changes since it last loaded or saved:
    /// rank gained per folder, latest visit, folders it forgot; then aged.
    fn merged(&self, mut disk: Self, now: u64) -> Self {
        let base = |path: &Path| self.saved.iter().find(|f| f.path == path);
        let mine = |path: &Path| self.folders.iter().find(|f| f.path == path);
        disk.folders
            .retain(|f| base(&f.path).is_none() || mine(&f.path).is_some());
        for folder in &self.folders {
            let before = base(&folder.path);
            let gained = folder.rank - before.map_or(0.0, |b| b.rank);
            let visited = before.is_none_or(|b| folder.last_visit > b.last_visit);
            match disk.folders.iter_mut().find(|f| f.path == folder.path) {
                Some(on_disk) => {
                    on_disk.rank += gained.max(0.0);
                    on_disk.last_visit = on_disk.last_visit.max(folder.last_visit);
                }
                // Gone from disk (aged out, or forgotten by another
                // instance): back only if visited here since.
                None if visited => disk.folders.push(folder.clone()),
                None => {}
            }
        }
        disk.age(now);
        disk
    }

    /// Records a visit of `path` at `now` (seconds since the epoch).
    pub fn visit(&mut self, path: &Path, now: u64) {
        match self.folders.iter_mut().find(|f| f.path == path) {
            Some(folder) => {
                folder.rank += 1.0;
                folder.last_visit = now;
            }
            None => self.folders.push(FolderVisits {
                path: path.to_path_buf(),
                rank: 1.0,
                last_visit: now,
            }),
        }
        self.age(now);
    }

    /// Forgets `path` (a folder that no longer exists).
    pub fn remove(&mut self, path: &Path) {
        self.folders.retain(|f| f.path != path);
    }

    /// Forgets every folder `gone` says no longer exists; returns how many.
    pub fn prune(&mut self, gone: impl Fn(&Path) -> bool) -> usize {
        let before = self.folders.len();
        self.folders.retain(|f| !gone(&f.path));
        before - self.folders.len()
    }

    /// Shrinks the ranks once they add up to [`MAX_TOTAL_RANK`] (dropping
    /// folders whose rank falls under one) and keeps the best
    /// [`MAX_FOLDERS`].
    fn age(&mut self, now: u64) {
        let total: f64 = self.folders.iter().map(|f| f.rank).sum();
        if total > MAX_TOTAL_RANK {
            let factor = AGED_SHARE * MAX_TOTAL_RANK / total;
            for folder in &mut self.folders {
                folder.rank *= factor;
            }
            self.folders.retain(|f| f.rank >= 1.0);
        }
        if self.folders.len() > MAX_FOLDERS {
            self.folders
                .sort_by(|a, b| b.score(now).total_cmp(&a.score(now)));
            self.folders.truncate(MAX_FOLDERS);
        }
    }

    /// The folders matching `query` (fuzzy, ignoring case; every word must
    /// match), best first: folders whose own name matches the last word,
    /// then by score. An empty query lists every folder by score.
    pub fn ranked(&self, query: &str, now: u64) -> Vec<FolderMatch> {
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let whole = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let last = words
            .last()
            .map(|w| Pattern::parse(w, CaseMatching::Ignore, Normalization::Smart));
        let mut buf = Vec::new();
        let mut found: Vec<FolderMatch> = self
            .folders
            .iter()
            .filter_map(|folder| {
                let text = folder.path.to_string_lossy();
                if !words.is_empty() {
                    whole.score(Utf32Str::new(&text, &mut buf), &mut matcher)?;
                }
                let name = crate::fs::file_name_lossy(&folder.path);
                let name_hit = last.as_ref().is_some_and(|last| {
                    last.score(Utf32Str::new(&name, &mut buf), &mut matcher)
                        .is_some()
                });
                Some(FolderMatch {
                    path: folder.path.clone(),
                    score: folder.score(now),
                    name_hit,
                })
            })
            .collect();
        found.sort_by(|a, b| {
            b.name_hit
                .cmp(&a.name_hit)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.path.cmp(&b.path))
        });
        found
    }

    /// The best folder for `query` that `usable` accepts (`z <query>`).
    pub fn best(&self, query: &str, now: u64, usable: impl Fn(&Path) -> bool) -> Option<PathBuf> {
        self.ranked(query, now)
            .into_iter()
            .find(|m| usable(&m.path))
            .map(|m| m.path)
    }
}

/// Seconds since the Unix epoch (0 before it).
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn default_path() -> PathBuf {
    super::paths::get_config_dir().join(FILE_NAME)
}

#[cfg(test)]
mod tests;
