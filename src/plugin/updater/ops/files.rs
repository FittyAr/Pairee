//! Plugin files of an install: download (size-limited) and SHA-256 check in
//! memory, atomic writes, and removal of files the new version dropped.

use super::install_guard::{safe_join, verify_bytes_sha256};
use crate::plugin::updater::http::{self, MAX_FILE_BYTES, MAX_PLUGIN_BYTES, MAX_PLUGIN_FILES};
use crate::plugin::updater::registry::plugin_file_url;
use crate::plugin::updater::report::Report;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One verified file, ready to be written.
pub struct VerifiedFile {
    pub rel_path: String,
    pub sha256: String,
    pub dest: PathBuf,
    pub bytes: Vec<u8>,
}

impl VerifiedFile {
    /// `bytes` already trusted (the signed manifest) as `rel_path`.
    pub fn new(plugin_dir: &Path, rel_path: &str, bytes: Vec<u8>) -> Result<Self> {
        Ok(Self {
            rel_path: rel_path.to_string(),
            sha256: crate::update::downloader::sha256_hex(&bytes),
            dest: safe_join(plugin_dir, rel_path)?,
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
    let mut targets = Vec::with_capacity(files.len());
    for (rel_path, sha256) in files {
        targets.push((rel_path, sha256, safe_join(plugin_dir, rel_path)?));
    }
    let client = http::client()?;
    let mut total = 0usize;
    let mut verified = Vec::with_capacity(targets.len());
    for (rel_path, sha256, dest) in targets {
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
            dest,
            bytes,
        });
    }
    Ok(verified)
}

/// Writes the verified files (each atomically) and returns the new
/// `relative path → sha256` map for the lockfile.
pub fn write_all(files: Vec<VerifiedFile>) -> Result<HashMap<String, String>> {
    let mut written = HashMap::new();
    for file in files {
        crate::config::write_atomic(&file.dest, &file.bytes)?;
        written.insert(file.rel_path, file.sha256);
    }
    Ok(written)
}

/// Removes the files the previous version had (`old`) that the new one
/// (`new`) no longer lists, then any folder they leave empty. Only paths
/// recorded in the lockfile are touched, through the same `safe_join`
/// check as installs. A path the new version lists in another case is
/// kept: on a case-insensitive file system it is the file just written.
pub fn remove_stale(
    plugin_dir: &Path,
    old: &HashMap<String, String>,
    new: &HashMap<String, String>,
    report: &mut Report,
) {
    let dropped = |rel: &&String| !new.keys().any(|n| n.eq_ignore_ascii_case(rel));
    for rel_path in old.keys().filter(dropped) {
        let Ok(path) = safe_join(plugin_dir, rel_path) else {
            continue;
        };
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
        let file = VerifiedFile::new(dir.path(), "manifest.toml", b"name = \"x\"".to_vec()).unwrap();
        assert_eq!(
            file.sha256,
            crate::update::downloader::sha256_hex(b"name = \"x\"")
        );
        assert!(VerifiedFile::new(dir.path(), "../m.toml", Vec::new()).is_err());
    }

    #[test]
    fn write_all_writes_and_records_hashes() {
        let dir = tempfile::tempdir().unwrap();
        let files = vec![VerifiedFile {
            rel_path: "lib/a.lua".into(),
            sha256: "abc".into(),
            dest: dir.path().join("lib").join("a.lua"),
            bytes: b"return 1".to_vec(),
        }];
        let written = write_all(files).unwrap();
        assert_eq!(written.get("lib/a.lua").map(String::as_str), Some("abc"));
        assert_eq!(
            std::fs::read(dir.path().join("lib").join("a.lua")).unwrap(),
            b"return 1"
        );
    }
}
