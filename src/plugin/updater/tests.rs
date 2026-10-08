//! Offline tests for the plugin updater: registry/manifest parsing, update
//! detection, registry URLs and lockfile persistence. No network access.

use super::lockfile::{read_lockfile_from, write_lockfile_to};
use super::registry::{parse_blocklist, parse_index, plugin_file_url, registry_author};
use super::types::{PinnedPlugin, PluginsLock, RegistryPluginManifestWrapper};
use std::collections::HashMap;

const INDEX: &str = r#"
[plugins.git-status]
name = "git-status"
version = "1.2.0"
description = "Shows git status"
author = "Alice"
languages = ["en", "es"]
hooks = ["on_dir_change"]
min_pairee = "0.8.0"

[plugins.minimal]
name = "minimal"
version = "0.1.0"
"#;

const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

fn pinned(version: &str, pinned: bool, files: &[(&str, &str)]) -> PinnedPlugin {
    PinnedPlugin {
        version: version.to_string(),
        pinned,
        files: files
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    }
}

#[test]
fn index_parses_full_and_minimal_entries() {
    let index = parse_index(INDEX).unwrap();
    assert_eq!(index.plugins.len(), 2);

    let full = &index.plugins["git-status"];
    assert_eq!(full.version, "1.2.0");
    assert_eq!(full.author.as_deref(), Some("Alice"));
    assert_eq!(
        full.languages.as_deref(),
        Some(&["en".into(), "es".into()][..])
    );
    assert_eq!(full.hooks.as_deref(), Some(&["on_dir_change".into()][..]));
    assert_eq!(full.min_pairee.as_deref(), Some("0.8.0"));

    let minimal = &index.plugins["minimal"];
    assert!(minimal.description.is_none());
    assert!(minimal.author.is_none());
}

#[test]
fn index_rejects_malformed_toml_and_missing_fields() {
    assert!(parse_index("plugins = [").is_err());
    assert!(
        parse_index("[plugins.x]\nname = \"x\"\n").is_err(),
        "version is required"
    );
}

#[test]
fn blocklist_parses_and_tolerates_garbage() {
    let list = parse_blocklist("[blocked]\nevil = \"steals tokens\"\n");
    assert_eq!(list.blocked["evil"], "steals tokens");
    assert!(parse_blocklist("not toml ===").blocked.is_empty());
    assert!(parse_blocklist("").blocked.is_empty());
}

#[test]
fn manifest_parses_plugin_and_files_sections() {
    let text = format!(
        "[plugin]\nname = \"demo\"\nversion = \"2.0.0\"\n\n[files]\n\"main.lua\" = \"{HELLO_SHA256}\"\n\"lib/util.lua\" = \"abc\"\n"
    );
    let manifest: RegistryPluginManifestWrapper = toml::from_str(&text).unwrap();
    assert_eq!(manifest.plugin.unwrap().version, "2.0.0");
    let files = manifest.files.unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files["main.lua"], HELLO_SHA256);
}

#[test]
fn manifest_without_files_section_has_none() {
    let manifest: RegistryPluginManifestWrapper =
        toml::from_str("[plugin]\nname = \"demo\"\nversion = \"1.0.0\"\n").unwrap();
    assert!(manifest.files.is_none());
}

#[test]
fn update_detection_compares_against_registry_version() {
    let index = parse_index(INDEX).unwrap();
    assert_eq!(index.update_for("git-status", "1.1.9"), Some("1.2.0"));
    assert_eq!(index.update_for("git-status", "1.2.0"), None);
    // The registry only lists the latest release: a different (even lower)
    // version is still offered so users converge on the published one.
    assert_eq!(index.update_for("git-status", "9.0.0"), Some("1.2.0"));
    assert_eq!(index.update_for("not-in-registry", "1.0.0"), None);
}

#[test]
fn registry_author_defaults_blank_values() {
    assert_eq!(registry_author(Some("  Alice ")), "Alice");
    assert_eq!(registry_author(Some("   ")), "unknown");
    assert_eq!(registry_author(None), "unknown");
}

#[test]
fn plugin_file_urls_are_sharded_by_author_initial() {
    let url = plugin_file_url("Alice", "git-status", "manifest.toml");
    assert!(
        url.ends_with("/registry/plugins/a/Alice/git-status/manifest.toml"),
        "{url}"
    );
    assert!(url.starts_with("https://"), "{url}");

    let digit = plugin_file_url("42team", "demo", "lib/util.lua");
    assert!(
        digit.ends_with("/plugins/_/42team/demo/lib/util.lua"),
        "{digit}"
    );
}

#[test]
fn lockfile_round_trips_through_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plugins.lock");

    let mut plugins = HashMap::new();
    plugins.insert(
        "git-status".to_string(),
        pinned("1.2.0", true, &[("main.lua", HELLO_SHA256)]),
    );
    plugins.insert("minimal".to_string(), pinned("0.1.0", false, &[]));
    write_lockfile_to(&path, &PluginsLock { plugins }).unwrap();

    let lock = read_lockfile_from(&path);
    assert_eq!(lock.plugins.len(), 2);
    let gs = &lock.plugins["git-status"];
    assert_eq!(gs.version, "1.2.0");
    assert!(gs.pinned);
    assert_eq!(gs.files["main.lua"], HELLO_SHA256);
    assert!(lock.plugins["minimal"].files.is_empty());
}

#[test]
fn lockfile_missing_or_corrupt_reads_as_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("plugins.lock");
    assert!(read_lockfile_from(&path).plugins.is_empty());

    std::fs::write(&path, "plugins = 3").unwrap();
    assert!(read_lockfile_from(&path).plugins.is_empty());
}

#[test]
fn lockfile_hashes_match_installed_file_contents() {
    // `pairee plugin verify` recomputes each file's SHA-256 and compares it
    // (case-insensitively) with the lockfile entry.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("main.lua");
    std::fs::write(&file, b"hello").unwrap();

    let actual = crate::update::downloader::compute_sha256(&file).unwrap();
    assert!(actual.eq_ignore_ascii_case(HELLO_SHA256), "{actual}");

    std::fs::write(&file, b"tampered").unwrap();
    let tampered = crate::update::downloader::compute_sha256(&file).unwrap();
    assert!(!tampered.eq_ignore_ascii_case(HELLO_SHA256));
}
