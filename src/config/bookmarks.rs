//! Persistent folder shortcuts (Ctrl+Alt+1…9) and the directory hotlist,
//! stored in `<config_dir>/bookmarks.toml`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

const FILE_NAME: &str = "bookmarks.toml";

/// Lowest and highest folder shortcut slot.
pub const SHORTCUT_SLOTS: std::ops::RangeInclusive<u8> = 1..=9;

/// One named entry of the directory hotlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotlistEntry {
    pub name: String,
    pub path: PathBuf,
}

/// On-disk representation of `bookmarks.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookmarksFile {
    /// Slot number (as a string key, TOML tables need string keys) → folder.
    #[serde(default)]
    pub shortcuts: BTreeMap<String, PathBuf>,
    /// `None` until the user edits the hotlist; the localized defaults are used meanwhile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotlist: Option<Vec<HotlistEntry>>,
}

impl BookmarksFile {
    /// Loads `bookmarks.toml` from the config directory (default on missing/invalid file).
    pub fn load() -> Self {
        Self::load_from(&default_path())
    }

    pub fn load_from(path: &Path) -> Self {
        super::toml_store::load_or_default(path)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&default_path())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        super::toml_store::save(path, self)
    }

    /// Folder shortcuts as slot → path, ignoring malformed or out-of-range keys.
    pub fn shortcut_map(&self) -> HashMap<u8, PathBuf> {
        self.shortcuts
            .iter()
            .filter_map(|(k, v)| {
                let n = k.parse::<u8>().ok()?;
                SHORTCUT_SLOTS.contains(&n).then(|| (n, v.clone()))
            })
            .collect()
    }

    pub fn set_shortcuts(&mut self, map: &HashMap<u8, PathBuf>) {
        self.shortcuts = map
            .iter()
            .filter(|(n, _)| SHORTCUT_SLOTS.contains(n))
            .map(|(n, p)| (n.to_string(), p.clone()))
            .collect();
    }

    /// The user's hotlist, or the localized defaults if it was never edited.
    pub fn hotlist_entries(&self) -> Vec<HotlistEntry> {
        self.hotlist.clone().unwrap_or_else(default_hotlist)
    }
}

/// Persists the folder shortcuts, keeping the hotlist untouched.
pub fn save_shortcuts(map: &HashMap<u8, PathBuf>) -> Result<()> {
    let mut file = BookmarksFile::load();
    file.set_shortcuts(map);
    file.save()
}

/// Persists the hotlist, keeping the folder shortcuts untouched.
pub fn save_hotlist(entries: &[HotlistEntry]) -> Result<()> {
    let mut file = BookmarksFile::load();
    file.hotlist = Some(entries.to_vec());
    file.save()
}

fn default_path() -> PathBuf {
    crate::config::paths::get_config_dir().join(FILE_NAME)
}

/// Localized default hotlist: home, desktop, documents, downloads and filesystem root.
pub fn default_hotlist() -> Vec<HotlistEntry> {
    use crate::config::localization::t;
    let mut entries = Vec::new();
    let mut push = |key: &str, path: Option<PathBuf>| {
        if let Some(path) = path {
            entries.push(HotlistEntry { name: t(key), path });
        }
    };
    let dirs = directories::UserDirs::new();
    push(
        "hotlist_home",
        dirs.as_ref().map(|u| u.home_dir().to_path_buf()),
    );
    push(
        "hotlist_desktop",
        dirs.as_ref()
            .and_then(|u| u.desktop_dir().map(Path::to_path_buf)),
    );
    push(
        "hotlist_documents",
        dirs.as_ref()
            .and_then(|u| u.document_dir().map(Path::to_path_buf)),
    );
    push(
        "hotlist_downloads",
        dirs.as_ref()
            .and_then(|u| u.download_dir().map(Path::to_path_buf)),
    );
    push("hotlist_system_root", Some(filesystem_root()));
    entries
}

fn filesystem_root() -> PathBuf {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| cwd.ancestors().last().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

/// Display name for a new hotlist entry: the folder's own name, or the full path for roots.
pub fn entry_name_for(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_roundtrip_through_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let mut map = HashMap::new();
        map.insert(1u8, PathBuf::from("/a"));
        map.insert(9u8, PathBuf::from("/b"));
        let mut file = BookmarksFile::default();
        file.set_shortcuts(&map);
        file.hotlist = Some(vec![HotlistEntry {
            name: "x".into(),
            path: PathBuf::from("/x"),
        }]);
        file.save_to(&path).unwrap();

        let loaded = BookmarksFile::load_from(&path);
        assert_eq!(loaded.shortcut_map(), map);
        assert_eq!(loaded.hotlist_entries().len(), 1);
    }

    #[test]
    fn out_of_range_and_bad_keys_are_ignored() {
        let file: BookmarksFile =
            toml::from_str("[shortcuts]\n\"0\" = \"/z\"\n\"3\" = \"/c\"\nfoo = \"/f\"\n").unwrap();
        let map = file.shortcut_map();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&3), Some(&PathBuf::from("/c")));
    }

    #[test]
    fn missing_or_invalid_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        assert_eq!(BookmarksFile::load_from(&path), BookmarksFile::default());
        std::fs::write(&path, "not = [valid").unwrap();
        assert_eq!(BookmarksFile::load_from(&path), BookmarksFile::default());
    }

    #[test]
    fn unedited_hotlist_uses_defaults_with_root() {
        let entries = BookmarksFile::default().hotlist_entries();
        assert!(entries.iter().any(|e| e.path.parent().is_none()));
    }

    #[test]
    fn entry_name_prefers_folder_name() {
        assert_eq!(entry_name_for(Path::new("/home/user/src")), "src");
        let root = filesystem_root();
        assert_eq!(entry_name_for(&root), root.to_string_lossy());
    }
}
