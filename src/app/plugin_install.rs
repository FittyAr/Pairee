//! Installing a plugin under development into the user's plugins folder
//! (the "install dev plugin" action and the developer menu share it): its
//! files replace the installed copy and are recorded, hashed, in the
//! lockfile so the plugin loads as installed.

use crate::config::localization::t;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// A finished local install.
#[derive(Debug)]
pub struct LocalInstall {
    pub name: String,
    pub dest: PathBuf,
    /// Files copied.
    pub files: usize,
}

/// What an install is doing, for a progress display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStep<'a> {
    Copying(&'a str),
    Hashing(&'a str),
}

/// `name` is one plain path component (no folder separators, `.` or `..`).
pub fn is_plain_name(name: &str) -> bool {
    let mut parts = Path::new(name).components();
    matches!(
        (parts.next(), parts.next()),
        (Some(Component::Normal(_)), None)
    )
}

/// Installs the plugin in `src` (a folder with `manifest.toml`). A stale
/// installed copy is replaced, and any file that cannot be copied fails
/// the install (the lockfile then records nothing). `on_step` gets each
/// step with its position and the step count.
pub fn install_local(
    src: &Path,
    on_step: &dyn Fn(InstallStep<'_>, usize, usize),
) -> Result<LocalInstall, String> {
    let text = std::fs::read_to_string(src.join("manifest.toml")).map_err(|e| e.to_string())?;
    let manifest =
        crate::plugin::loader::PluginManifest::parse(&text).map_err(|e| format!("{e:?}"))?;
    if !is_plain_name(&manifest.name) {
        return Err(t("plugin_dev_init_bad_name").replace("{}", &manifest.name));
    }
    let dest = crate::config::paths::get_installed_plugin_dir(&manifest.name);
    let same = |a: &Path, b: &Path| matches!((a.canonicalize(), b.canonicalize()), (Ok(a), Ok(b)) if a == b);
    if same(src, &dest) {
        return Err(t("plugin_install_dev_into_itself"));
    }
    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    let files = crate::plugin::loader::get_plugin_files(src);
    let mut hashes = HashMap::new();
    for (idx, (rel, from)) in files.iter().enumerate() {
        on_step(InstallStep::Copying(rel), idx + 1, files.len());
        let to = dest.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::copy(from, &to).map_err(|e| format!("{rel}: {e}"))?;
    }
    for (idx, (rel, _)) in files.iter().enumerate() {
        on_step(InstallStep::Hashing(rel), idx + 1, files.len());
        let hash = crate::update::downloader::compute_sha256(&dest.join(rel))
            .map_err(|e| format!("{rel}: {e}"))?;
        hashes.insert(rel.clone(), hash);
    }
    let mut lock = crate::plugin::updater::read_lockfile();
    lock.plugins.insert(
        manifest.name.clone(),
        crate::plugin::updater::PinnedPlugin {
            version: manifest.version,
            pinned: false,
            files: hashes,
        },
    );
    crate::plugin::updater::write_lockfile(&lock).map_err(|e| e.to_string())?;
    Ok(LocalInstall {
        name: manifest.name,
        dest,
        files: files.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names_only() {
        assert!(is_plain_name("demo.pairee"));
        for bad in ["../demo", "a/b", "..", ".", ""] {
            assert!(!is_plain_name(bad), "{bad:?}");
        }
    }

    #[test]
    fn install_replaces_the_copy_and_records_every_file() {
        let root = tempfile::tempdir().unwrap();
        let _redirect = crate::config::paths::test_root::redirect(root.path());
        let src = root.path().join("dev").join("demo.pairee");
        std::fs::create_dir_all(src.join("lang")).unwrap();
        std::fs::write(
            src.join("manifest.toml"),
            "name = \"demo\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        std::fs::write(src.join("main.lua"), "-- v1").unwrap();
        std::fs::write(src.join("lang").join("en.toml"), "x = \"y\"").unwrap();
        let dest = crate::config::paths::get_installed_plugin_dir("demo");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("stale.lua"), "old").unwrap();

        let done = install_local(&src, &|_, _, _| {}).expect("installed");
        assert_eq!(done.files, 3);
        assert!(!dest.join("stale.lua").exists(), "stale files are gone");
        assert!(dest.join("lang").join("en.toml").exists());
        let lock = crate::plugin::updater::read_lockfile();
        assert_eq!(lock.plugins["demo"].files.len(), 3);
    }
}
