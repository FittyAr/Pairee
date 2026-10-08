//! Folder tree of an archive built from its entry list, so browsing it is
//! a map lookup. Unsafe names (`..`, absolute paths) are left out; folders
//! that only appear as path prefixes are created implicitly.

use super::format::{EntryKind, EntryMeta};
use super::safe_extract::ExtractGuard;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// One entry of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub is_dir: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

impl Node {
    const DIR: Self = Self {
        is_dir: true,
        is_symlink: false,
        size: 0,
        modified: None,
    };
}

/// Children of every folder, keyed by the folder's path inside the archive
/// (the empty path is the archive root).
#[derive(Debug, Default)]
pub struct ArchiveIndex {
    dirs: HashMap<PathBuf, BTreeMap<String, Node>>,
}

impl ArchiveIndex {
    pub fn build(entries: &[EntryMeta]) -> Self {
        let mut index = Self::default();
        index.dirs.insert(PathBuf::new(), BTreeMap::new());
        for entry in entries {
            let Ok(rel) = ExtractGuard::sanitize(&entry.name) else {
                continue;
            };
            if rel.as_os_str().is_empty() {
                continue;
            }
            index.insert(&rel, node_of(entry));
        }
        index
    }

    fn insert(&mut self, rel: &Path, node: Node) {
        let mut parent = PathBuf::new();
        let parts: Vec<_> = rel.components().collect();
        for (i, part) in parts.iter().enumerate() {
            let name = part.as_os_str().to_string_lossy().into_owned();
            let here = parent.join(&name);
            let last = i + 1 == parts.len();
            let children = self.dirs.entry(parent).or_default();
            if last {
                // A later entry with the same name wins (tar updates), but a
                // folder never turns back into a file through an implicit one.
                children.insert(name, node.clone());
            } else {
                children.entry(name).or_insert(Node::DIR);
            }
            if !last || node.is_dir {
                self.dirs.entry(here.clone()).or_default();
            }
            parent = here;
        }
    }

    /// Children of the folder `dir` (relative to the archive root).
    pub fn children(&self, dir: &Path) -> Option<&BTreeMap<String, Node>> {
        self.dirs.get(dir)
    }

    /// The entry at `rel`; the root is a folder.
    pub fn node(&self, rel: &Path) -> Option<Node> {
        let Some(name) = rel.file_name() else {
            return Some(Node::DIR);
        };
        let parent = rel.parent().unwrap_or(Path::new(""));
        self.dirs
            .get(parent)?
            .get(name.to_string_lossy().as_ref())
            .cloned()
    }
}

fn node_of(entry: &EntryMeta) -> Node {
    let is_dir = entry.kind == EntryKind::Dir;
    Node {
        is_dir,
        is_symlink: matches!(entry.kind, EntryKind::Symlink(_)),
        size: if is_dir { 0 } else { entry.size },
        modified: entry.modified,
    }
}
