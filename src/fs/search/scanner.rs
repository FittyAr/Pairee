//! "Find files": walks the tree below the search root with the shared
//! [`Walker`] on a blocking thread and sends every match. The search stops
//! as soon as the receiver is dropped (the dialog was closed or a new
//! search started).

use super::content::file_contains;
use super::types::{SearchQuery, SearchTarget};
use crate::fs::vfs::LocalVfs;
use crate::fs::walk::{WalkOptions, Walker};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Upper bound on the depth of the recursive directory walk.
pub const MAX_SEARCH_DEPTH: usize = 32;

/// Spawns a background task that searches for files and folders matching `query`.
pub fn find_files(query: SearchQuery) -> mpsc::Receiver<(PathBuf, bool)> {
    let (tx, rx) = mpsc::channel(256);
    tokio::task::spawn_blocking(move || search(&query, &tx));
    rx
}

/// Walks the tree, sending each match until the receiver goes away.
fn search(query: &SearchQuery, tx: &mpsc::Sender<(PathBuf, bool)>) {
    let options = WalkOptions {
        max_depth: MAX_SEARCH_DEPTH,
        ..WalkOptions::default()
    };
    let cancelled = || tx.is_closed();
    let walker = Walker::new(&LocalVfs, &query.root, options).with_cancel(&cancelled);
    // Unreadable folders are left out of a search.
    for item in walker.flatten() {
        let entry = &item.entry;
        if !wanted(query, entry.is_dir) || !name_matches(query, &entry.name) {
            continue;
        }
        let matches = match (&query.content, entry.is_dir) {
            (None, _) => true,
            (Some(_), true) => false,
            (Some(needle), false) => {
                file_contains(&entry.path, needle, query.case_sensitive, &cancelled)
            }
        };
        if matches
            && tx
                .blocking_send((entry.path.clone(), entry.is_dir))
                .is_err()
        {
            return;
        }
    }
}

fn wanted(query: &SearchQuery, is_dir: bool) -> bool {
    match query.target {
        SearchTarget::Any => true,
        SearchTarget::File => !is_dir,
        SearchTarget::Directory => is_dir,
    }
}

fn name_matches(query: &SearchQuery, name: &str) -> bool {
    query.name_glob.is_empty()
        || crate::fs::glob::glob_matches_case(&query.name_glob, name, query.case_sensitive)
}
