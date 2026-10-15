//! Installing a plugin under development into the user's plugins folder
//! (the "install dev plugin" action and the developer menu share it): its
//! files replace the installed copy and are recorded, hashed, in the
//! lockfile so the plugin loads as installed. The new copy is built beside
//! the installed one and swapped in only when complete, so a failed install
//! leaves the working plugin as it was.

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
    // The name becomes folder names (`<name>.pairee`, its data folder) and a
    // config key: the same `[A-Za-z0-9_-]` rule registry installs use.
    if !crate::plugin::updater::is_identifier(&manifest.name) {
        return Err(t("plugin_dev_init_bad_name").replace("{}", &manifest.name));
    }
    let dest = crate::config::paths::get_installed_plugin_dir(&manifest.name);
    if overlaps(src, &dest) {
        return Err(t("plugin_install_dev_into_itself"));
    }
    let mut lock = crate::plugin::updater::try_read_lockfile().map_err(|e| format!("{e:#}"))?;
    let staging = crate::plugin::updater::ops::staging_dir(&dest);
    let built = build_copy(src, &staging, on_step);
    let hashes = match built {
        Ok(hashes) => hashes,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
    };
    let files = hashes.len();
    if let Err(e) = crate::plugin::updater::ops::replace_dir(&dest, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(format!("{}: {e}", dest.display()));
    }
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
        files,
    })
}

/// `true` when the folders are the same or one lies inside the other:
/// copying (or replacing) one would then delete or copy into the other.
fn overlaps(src: &Path, dest: &Path) -> bool {
    let canonical = |p: &Path| {
        // `dest` may not exist yet: resolve its nearest existing ancestor.
        let mut tail = PathBuf::new();
        let mut base = p.to_path_buf();
        loop {
            if let Ok(real) = base.canonicalize() {
                return real.join(tail);
            }
            let Some(name) = base.file_name().map(std::ffi::OsStr::to_owned) else {
                return p.to_path_buf();
            };
            tail = Path::new(&name).join(tail);
            if !base.pop() {
                return p.to_path_buf();
            }
        }
    };
    let (src, dest) = (canonical(src), canonical(dest));
    src.starts_with(&dest) || dest.starts_with(&src)
}

/// Copies the publishable files of `src` into the new folder `staging` and
/// returns their hashes by relative path.
fn build_copy(
    src: &Path,
    staging: &Path,
    on_step: &dyn Fn(InstallStep<'_>, usize, usize),
) -> Result<HashMap<String, String>, String> {
    let _ = std::fs::remove_dir_all(staging);
    std::fs::create_dir_all(staging).map_err(|e| format!("{}: {e}", staging.display()))?;
    let files = crate::plugin::loader::get_plugin_files(src);
    let mut hashes = HashMap::new();
    for (idx, (rel, from)) in files.iter().enumerate() {
        on_step(InstallStep::Copying(rel), idx + 1, files.len());
        let to = staging.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::copy(from, &to).map_err(|e| format!("{rel}: {e}"))?;
    }
    for (idx, (rel, _)) in files.iter().enumerate() {
        on_step(InstallStep::Hashing(rel), idx + 1, files.len());
        let hash = crate::update::downloader::compute_sha256(&staging.join(rel))
            .map_err(|e| format!("{rel}: {e}"))?;
        hashes.insert(rel.clone(), hash);
    }
    Ok(hashes)
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
    fn a_source_containing_or_inside_the_destination_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let dest = root.path().join("plugins").join("demo.pairee");
        std::fs::create_dir_all(dest.join("sub")).unwrap();
        assert!(overlaps(&dest, &dest));
        assert!(
            overlaps(&dest.join("sub"), &dest),
            "source inside the install"
        );
        assert!(overlaps(root.path(), &dest), "source holding the install");
        let elsewhere = root.path().join("dev").join("demo.pairee");
        std::fs::create_dir_all(&elsewhere).unwrap();
        assert!(!overlaps(&elsewhere, &dest));
    }

    #[test]
    fn a_name_that_is_not_an_identifier_is_refused_and_nothing_changes() {
        let root = tempfile::tempdir().unwrap();
        let _redirect = crate::config::paths::test_root::redirect(root.path());
        let src = root.path().join("dev");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("manifest.toml"),
            "name = \"my.plugin\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        assert!(install_local(&src, &|_, _, _| {}).is_err());
    }

    #[test]
    fn a_failed_copy_keeps_the_working_install() {
        let root = tempfile::tempdir().unwrap();
        let _redirect = crate::config::paths::test_root::redirect(root.path());
        let src = root.path().join("dev").join("demo.pairee");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("manifest.toml"),
            "name = \"demo\"\nversion = \"2.0.0\"\n",
        )
        .unwrap();
        let dest = crate::config::paths::get_installed_plugin_dir("demo");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("main.lua"), "-- working").unwrap();
        // A corrupt lockfile stops the install before anything is touched.
        std::fs::write(
            crate::config::paths::get_config_dir().join("plugins.lock"),
            "plugins = 3",
        )
        .unwrap();
        assert!(install_local(&src, &|_, _, _| {}).is_err());
        assert_eq!(
            std::fs::read_to_string(dest.join("main.lua")).unwrap(),
            "-- working"
        );
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
        let lock = crate::plugin::updater::try_read_lockfile().unwrap();
        assert_eq!(lock.plugins["demo"].files.len(), 3);
    }
}
