//! Plugin files of an install: download (size-limited) and SHA-256 check in
//! memory, an atomic swap of the finished folder, and removal of files the
//! new version dropped.

use super::install_guard::{check_unique_keys, safe_join, verify_bytes_sha256};
use crate::plugin::updater::http::{self, MAX_FILE_BYTES, MAX_PLUGIN_BYTES, MAX_PLUGIN_FILES};
use crate::plugin::updater::registry::plugin_file_url;
use crate::plugin::updater::report::Report;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// One verified file, ready to be written.
pub struct VerifiedFile {
    pub rel_path: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}

impl VerifiedFile {
    /// `bytes` already trusted (the signed manifest) as `rel_path`.
    pub fn new(plugin_dir: &Path, rel_path: &str, bytes: Vec<u8>) -> Result<Self> {
        safe_join(plugin_dir, rel_path)?;
        Ok(Self {
            rel_path: rel_path.to_string(),
            sha256: crate::update::downloader::sha256_hex(&bytes),
            bytes,
        })
    }
}

/// Downloads and verifies every `[files]` entry of the signed manifest
/// before anything is written, so a tampered or oversized plugin never
/// reaches the disk and no partial install is left.
pub async fn download_verified(
    author: &str,
    name: &str,
    files: &HashMap<String, String>,
    plugin_dir: &Path,
    report: &mut Report,
) -> Result<Vec<VerifiedFile>> {
    if files.len() > MAX_PLUGIN_FILES {
        anyhow::bail!(
            "Plugin lists {} files (limit {MAX_PLUGIN_FILES})",
            files.len()
        );
    }
    // Reject hostile `[files]` keys before any network fetch.
    check_unique_keys(files.keys())?;
    let mut targets = Vec::with_capacity(files.len());
    for (rel_path, sha256) in files {
        safe_join(plugin_dir, rel_path)?;
        targets.push((rel_path, sha256));
    }
    let client = http::client()?;
    let mut total = 0usize;
    let mut verified = Vec::with_capacity(targets.len());
    for (rel_path, sha256) in targets {
        let url = plugin_file_url(author, name, rel_path);
        let bytes = http::fetch_required(&client, &url, MAX_FILE_BYTES).await?;
        total += bytes.len();
        if total > MAX_PLUGIN_BYTES {
            anyhow::bail!("Plugin is larger than the {MAX_PLUGIN_BYTES}-byte limit");
        }
        verify_bytes_sha256(&bytes, sha256)
            .map_err(|e| anyhow::anyhow!("Verification failed for file '{rel_path}': {e:#}"))?;
        report.line(format!("  ✓ {rel_path} verified."));
        verified.push(VerifiedFile {
            rel_path: rel_path.clone(),
            sha256: sha256.clone(),
            bytes,
        });
    }
    Ok(verified)
}

/// Installs `files` into `plugin_dir` **atomically**: the current folder
/// (if any) is copied beside it, the new files are written there and the
/// files the previous version (`old`) shipped and this one dropped are
/// removed, and only then is the finished folder swapped in. A failure or a
/// crash before the swap leaves the working install untouched. Returns the
/// new `relative path → sha256` map for the lockfile.
pub fn install_staged(
    plugin_dir: &Path,
    files: Vec<VerifiedFile>,
    old: Option<&HashMap<String, String>>,
    report: &mut Report,
) -> Result<HashMap<String, String>> {
    let staging = staging_dir(plugin_dir);
    let _ = std::fs::remove_dir_all(&staging);
    let staged = stage(plugin_dir, &staging, files, old, report);
    let result = staged.and_then(|written| replace_dir(plugin_dir, &staging).map(|()| written));
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

/// Where a new copy of `plugin_dir` is built before it replaces it.
pub(crate) fn staging_dir(plugin_dir: &Path) -> PathBuf {
    sibling(plugin_dir, "staging")
}

/// `<plugin_dir>.<suffix>`: a sibling that is not a `.pairee` folder, so the
/// loader never mistakes it for a plugin.
fn sibling(plugin_dir: &Path, suffix: &str) -> PathBuf {
    let mut name = plugin_dir.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{suffix}"));
    plugin_dir.with_file_name(name)
}

fn stage(
    plugin_dir: &Path,
    staging: &Path,
    files: Vec<VerifiedFile>,
    old: Option<&HashMap<String, String>>,
    report: &mut Report,
) -> Result<HashMap<String, String>> {
    if plugin_dir.is_dir() {
        copy_tree(plugin_dir, staging)?;
    } else {
        std::fs::create_dir_all(staging)?;
    }
    let written = write_all(staging, files)?;
    if let Some(old) = old {
        remove_stale(staging, old, &written, report);
    }
    Ok(written)
}

/// Replaces `plugin_dir` by `staging`; the old folder is put back if the
/// second rename fails.
pub(crate) fn replace_dir(plugin_dir: &Path, staging: &Path) -> Result<()> {
    let backup = sibling(plugin_dir, "old");
    let _ = std::fs::remove_dir_all(&backup);
    let had_old = plugin_dir.exists();
    if had_old {
        std::fs::rename(plugin_dir, &backup)?;
    }
    if let Err(e) = std::fs::rename(staging, plugin_dir) {
        if had_old {
            let _ = std::fs::rename(&backup, plugin_dir);
        }
        return Err(e.into());
    }
    let _ = std::fs::remove_dir_all(&backup);
    Ok(())
}

/// Copies the folder `from` to `to` (links are not followed or copied).
pub(crate) fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Writes the verified files below `root` (each atomically) and returns the
/// new `relative path → sha256` map for the lockfile.
pub fn write_all(root: &Path, files: Vec<VerifiedFile>) -> Result<HashMap<String, String>> {
    let mut written = HashMap::new();
    for file in files {
        crate::config::write_atomic(&safe_join(root, &file.rel_path)?, &file.bytes)?;
        written.insert(file.rel_path, file.sha256);
    }
    Ok(written)
}

/// Every file below `dir` as a `/`-separated path relative to it (links and
/// hidden files included: they are code or data the plugin would see).
pub fn list_files(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(std::result::Result::ok) {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => walk(root, &path, out),
                Ok(_) => {
                    if let Ok(rel) = path.strip_prefix(root) {
                        let parts: Vec<_> = rel.iter().map(|p| p.to_string_lossy()).collect();
                        out.push(parts.join("/"));
                    }
                }
                Err(_) => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// Removes the files the previous version had (`old`) that the new one
/// (`new`) no longer lists, then any folder they leave empty. Only paths
/// recorded in the lockfile are touched, through the same `safe_join`
/// check as installs. A dropped path that is the very file the new version
/// wrote (same file under another case on a case-insensitive file system) is
/// kept; on a case-sensitive one the two are different files and the old one
/// goes.
pub fn remove_stale(
    plugin_dir: &Path,
    old: &HashMap<String, String>,
    new: &HashMap<String, String>,
    report: &mut Report,
) {
    let written: HashSet<PathBuf> = new
        .keys()
        .filter_map(|rel| safe_join(plugin_dir, rel).ok())
        .filter_map(|path| std::fs::canonicalize(path).ok())
        .collect();
    for rel_path in old.keys().filter(|rel| !new.contains_key(*rel)) {
        let Ok(path) = safe_join(plugin_dir, rel_path) else {
            continue;
        };
        if std::fs::canonicalize(&path).is_ok_and(|real| written.contains(&real)) {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => {
                report.line(format!("  - {rel_path} removed (no longer shipped)."));
                prune_empty_parents(plugin_dir, &path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => report.warn(format!("Could not remove stale file {rel_path}: {e}")),
        }
    }
}

/// Removes empty folders between `path` and `root` (exclusive).
fn prune_empty_parents(root: &Path, path: &Path) {
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d == root || !d.starts_with(root) || std::fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(keys: &[&str]) -> HashMap<String, String> {
        keys.iter()
            .map(|k| (k.to_string(), "00".to_string()))
            .collect()
    }

    #[test]
    fn stale_files_are_removed_and_kept_ones_stay() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("lib").join("old")).unwrap();
        for f in ["main.lua", "lib/old/gone.lua", "user-notes.txt"] {
            std::fs::write(root.join(f), "x").unwrap();
        }
        let mut report = Report::default();
        remove_stale(
            root,
            &map(&["main.lua", "lib/old/gone.lua", "../outside.lua"]),
            &map(&["main.lua"]),
            &mut report,
        );
        assert!(root.join("main.lua").exists());
        assert!(!root.join("lib/old/gone.lua").exists());
        assert!(!root.join("lib").exists(), "empty folders are pruned");
        assert!(
            root.join("user-notes.txt").exists(),
            "unknown files are kept"
        );
        assert_eq!(report.lines.len(), 1);
    }

    #[test]
    fn a_file_renamed_only_in_case_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("util.lua"), "new").unwrap();
        let mut report = Report::default();
        remove_stale(
            dir.path(),
            &map(&["Util.lua"]),
            &map(&["util.lua"]),
            &mut report,
        );
        assert!(dir.path().join("util.lua").exists());
    }

    #[test]
    fn the_signed_manifest_is_written_with_its_own_hash() {
        let dir = tempfile::tempdir().unwrap();
        let file =
            VerifiedFile::new(dir.path(), "manifest.toml", b"name = \"x\"".to_vec()).unwrap();
        assert_eq!(
            file.sha256,
            crate::update::downloader::sha256_hex(b"name = \"x\"")
        );
        assert!(VerifiedFile::new(dir.path(), "../m.toml", Vec::new()).is_err());
    }

    fn file(rel: &str, body: &str) -> VerifiedFile {
        VerifiedFile {
            rel_path: rel.into(),
            sha256: crate::update::downloader::sha256_hex(body.as_bytes()),
            bytes: body.as_bytes().to_vec(),
        }
    }

    #[test]
    fn install_swaps_the_finished_folder_in_and_keeps_user_files() {
        let parent = tempfile::tempdir().unwrap();
        let dir = parent.path().join("demo.pairee");
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("main.lua"), "old").unwrap();
        std::fs::write(dir.join("lib").join("gone.lua"), "x").unwrap();
        std::fs::write(dir.join("notes.txt"), "mine").unwrap();
        let mut report = Report::default();
        let written = install_staged(
            &dir,
            vec![file("main.lua", "new")],
            Some(&map(&["main.lua", "lib/gone.lua"])),
            &mut report,
        )
        .unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(
            std::fs::read_to_string(dir.join("main.lua")).unwrap(),
            "new"
        );
        assert!(!dir.join("lib").exists());
        assert!(dir.join("notes.txt").exists());
        let leftovers: Vec<_> = std::fs::read_dir(parent.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(leftovers, ["demo.pairee"], "no staging or backup is left");
    }

    #[test]
    fn a_failed_install_leaves_the_working_plugin_untouched() {
        let parent = tempfile::tempdir().unwrap();
        let dir = parent.path().join("demo.pairee");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.lua"), "old").unwrap();
        // `lib` is a file in the new version, but a folder cannot be made
        // under a file: the second write fails after the first succeeded.
        let files = vec![
            file("main.lua", "new"),
            file("a.lua", "1"),
            file("a.lua/b.lua", "2"),
        ];
        let mut report = Report::default();
        assert!(install_staged(&dir, files, None, &mut report).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join("main.lua")).unwrap(),
            "old"
        );
        assert!(!dir.join("a.lua").exists());
        assert_eq!(std::fs::read_dir(parent.path()).unwrap().count(), 1);
    }

    #[test]
    fn list_files_includes_hidden_and_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("lib")).unwrap();
        for f in ["main.lua", ".hidden", "lib/u.lua"] {
            std::fs::write(dir.path().join(f), "x").unwrap();
        }
        assert_eq!(list_files(dir.path()), [".hidden", "lib/u.lua", "main.lua"]);
    }

    #[test]
    fn write_all_writes_and_records_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![VerifiedFile {
            rel_path: "lib/a.lua".into(),
            sha256: "abc".into(),
            bytes: b"return 1".to_vec(),
        }];
        let written = write_all(dir.path(), files).unwrap();
        assert_eq!(written.get("lib/a.lua").map(String::as_str), Some("abc"));
        assert_eq!(
            std::fs::read(dir.path().join("lib").join("a.lua")).unwrap(),
            b"return 1"
        );
    }
}
