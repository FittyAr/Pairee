//! Linux and macOS: the binary in the release tarball replaces the running
//! one with an atomic rename (the running process keeps its old inode).
//!
//! `install.sh` keeps the program in `~/.local/lib/FittyAr/Pairee` with a
//! `~/.local/bin/pairee` link; the file behind a link is the one replaced, so
//! the link keeps working. A Linux install of the previous layout (the
//! program itself in `~/.local/bin`) is moved to the FittyAr folder by the
//! update, leaving a link in its place.

use anyhow::{Context as _, Result};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::{InstallResult, cleanup, extract};

/// Where the new binary goes.
#[derive(Debug, PartialEq, Eq)]
enum Target {
    /// Replace this file.
    Replace(PathBuf),
    /// Write the program here, then make `link` a symlink to it.
    MoveToLib { program: PathBuf, link: PathBuf },
}

/// Replaces the running binary with the one in the verified tarball.
pub(super) fn install(archive: &[u8]) -> Result<InstallResult> {
    let exe = live_exe_path(std::env::current_exe().context("cannot determine current exe path")?);
    let exe = resolve_link(&exe);
    if crate::fs::is_elevated() && !root_may_update(&exe) {
        anyhow::bail!(crate::config::localization::t("update_root_refused"));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let target = target_for(&exe, home.as_deref(), cfg!(target_os = "linux"));

    let new_bin = extract::file_from_tar_gz(archive, "pairee")?;

    match target {
        Target::Replace(path) => write_binary(&path, &new_bin)?,
        Target::MoveToLib { program, link } => {
            if let Err(e) = move_to_lib(&program, &link, &new_bin) {
                // The previous layout still works: update it where it is.
                log::warn!(
                    "Could not move Pairee to {}: {e:#}; updating {} in place",
                    program.display(),
                    link.display()
                );
                write_binary(&link, &new_bin)?;
            } else {
                log::info!(
                    "Moved Pairee to {} ({} links to it)",
                    program.display(),
                    link.display()
                );
            }
        }
    }
    Ok(InstallResult::RestartRequired)
}

/// Whether root (`sudo pairee`) may replace `exe`: only when root owns it
/// and its folder. The user's own copy (`~/.local/lib/FittyAr/Pairee`)
/// replaced by root would end up root-owned, and the user could no longer
/// update it; it is updated without sudo instead.
fn root_may_update(exe: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let owned_by_root = |p: &Path| std::fs::metadata(p).is_ok_and(|m| m.uid() == 0);
    owned_by_root(exe) && exe.parent().is_some_and(owned_by_root)
}

/// Linux reports a replaced binary as `<path> (deleted)`; the file to update
/// again is the one at `<path>`, which the previous update put there.
fn live_exe_path(exe: PathBuf) -> PathBuf {
    match exe.to_str().and_then(|s| s.strip_suffix(" (deleted)")) {
        Some(live) => PathBuf::from(live),
        None => exe,
    }
}

/// The file behind `exe` when it is a symlink (macOS reports the path it was
/// started through): renaming over the link would leave the program behind
/// it out of date.
fn resolve_link(exe: &Path) -> PathBuf {
    let is_link = std::fs::symlink_metadata(exe).is_ok_and(|m| m.file_type().is_symlink());
    if is_link && let Ok(real) = std::fs::canonicalize(exe) {
        return real;
    }
    exe.to_path_buf()
}

/// Where the update of the (link-resolved) binary `exe` goes. On Linux, a
/// program in `~/.local/bin` (the layout of `install.sh` before the FittyAr
/// folder) moves to `~/.local/lib/FittyAr/Pairee`.
fn target_for(exe: &Path, home: Option<&Path>, linux: bool) -> Target {
    let Some(home) = home.filter(|h| linux && h.is_absolute()) else {
        return Target::Replace(exe.to_path_buf());
    };
    let homes = [Some(home.to_path_buf()), std::fs::canonicalize(home).ok()];
    for home in homes.into_iter().flatten() {
        if exe == home.join(".local").join("bin").join("pairee") {
            return Target::MoveToLib {
                program: install_dir(&home).join("pairee"),
                link: exe.to_path_buf(),
            };
        }
    }
    Target::Replace(exe.to_path_buf())
}

/// The folder `install.sh` installs Pairee to.
fn install_dir(home: &Path) -> PathBuf {
    home.join(".local")
        .join("lib")
        .join("FittyAr")
        .join("Pairee")
}

/// Writes the program to `program`, then atomically replaces the file at
/// `link` with a symlink to it.
fn move_to_lib(program: &Path, link: &Path, new_bin: &[u8]) -> Result<()> {
    let dir = program.parent().context("program path has no folder")?;
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    write_binary(program, new_bin)?;

    let link_dir = link.parent().context("link has no folder")?;
    let staged = link_dir.join(format!(
        "{}link_{}",
        cleanup::STAGING_PREFIX,
        std::process::id()
    ));
    let _ = std::fs::remove_file(&staged);
    std::os::unix::fs::symlink(program, &staged)
        .with_context(|| format!("cannot create a link in {}", link_dir.display()))?;
    if let Err(e) = std::fs::rename(&staged, link) {
        let _ = std::fs::remove_file(&staged);
        return Err(e).with_context(|| format!("cannot replace {}", link.display()));
    }
    Ok(())
}

/// Writes `new_bin` to a randomly named sibling of `path` (created 0600 by
/// tempfile), marks it executable and atomically renames it over `path`.
fn write_binary(path: &Path, new_bin: &[u8]) -> Result<()> {
    let dir = path.parent().context("exe has no parent directory")?;
    let mut staged = tempfile::Builder::new()
        .prefix(cleanup::STAGING_PREFIX)
        .tempfile_in(dir)
        .context("cannot create staging file next to pairee binary")?;
    staged
        .write_all(new_bin)
        .context("failed to write new pairee binary")?;
    staged
        .as_file()
        .sync_all()
        .context("failed to flush new pairee binary")?;
    set_executable(staged.path())?;
    staged
        .persist(path)
        .map_err(|e| e.error)
        .context("failed to replace pairee binary")?;
    Ok(())
}

fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_updates_only_a_program_root_owns() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("pairee");
        std::fs::write(&exe, b"old").unwrap();
        // The test's own file: root's only when the tests run as root.
        assert_eq!(root_may_update(&exe), crate::fs::is_elevated());
        assert!(!root_may_update(&dir.path().join("missing")));
    }

    #[test]
    fn a_replaced_binary_maps_back_to_its_path() {
        assert_eq!(
            live_exe_path(PathBuf::from("/usr/local/bin/pairee (deleted)")),
            PathBuf::from("/usr/local/bin/pairee")
        );
        assert_eq!(
            live_exe_path(PathBuf::from("/usr/local/bin/pairee")),
            PathBuf::from("/usr/local/bin/pairee")
        );
    }

    #[test]
    fn the_fittyar_install_is_updated_in_place() {
        let exe = Path::new("/home/u/.local/lib/FittyAr/Pairee/pairee");
        assert_eq!(
            target_for(exe, Some(Path::new("/home/u")), true),
            Target::Replace(exe.to_path_buf())
        );
    }

    #[test]
    fn a_previous_local_bin_install_moves_to_the_fittyar_folder() {
        let exe = Path::new("/home/u/.local/bin/pairee");
        assert_eq!(
            target_for(exe, Some(Path::new("/home/u")), true),
            Target::MoveToLib {
                program: PathBuf::from("/home/u/.local/lib/FittyAr/Pairee/pairee"),
                link: exe.to_path_buf(),
            }
        );
        // Not on macOS, nor without a usable home.
        assert_eq!(
            target_for(exe, Some(Path::new("/home/u")), false),
            Target::Replace(exe.to_path_buf())
        );
        assert_eq!(
            target_for(exe, None, true),
            Target::Replace(exe.to_path_buf())
        );
        assert_eq!(
            target_for(exe, Some(Path::new("relative")), true),
            Target::Replace(exe.to_path_buf())
        );
    }

    #[test]
    fn other_locations_are_updated_in_place() {
        let home = Some(Path::new("/home/u"));
        for exe in [
            "/usr/local/bin/pairee",
            "/home/u/bin/pairee",
            "/home/other/.local/bin/pairee",
            "/home/u/.local/bin/pairee-dev",
        ] {
            assert_eq!(
                target_for(Path::new(exe), home, true),
                Target::Replace(PathBuf::from(exe)),
                "{exe}"
            );
        }
    }

    #[test]
    fn a_link_resolves_to_the_program_behind_it() {
        let dir = tempfile::tempdir().unwrap();
        let program = dir.path().join("lib").join("pairee");
        std::fs::create_dir_all(program.parent().unwrap()).unwrap();
        std::fs::write(&program, b"old").unwrap();
        let link = dir.path().join("pairee");
        std::os::unix::fs::symlink(&program, &link).unwrap();

        let real = std::fs::canonicalize(&program).unwrap();
        assert_eq!(resolve_link(&link), real);
        assert_eq!(resolve_link(&program), program);
    }

    #[test]
    fn moving_to_the_fittyar_folder_leaves_a_working_link() {
        let home = tempfile::tempdir().unwrap();
        let link = home.path().join(".local").join("bin").join("pairee");
        std::fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::fs::write(&link, b"old").unwrap();
        let program = install_dir(home.path()).join("pairee");

        move_to_lib(&program, &link, b"new").unwrap();

        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read(&link).unwrap(), b"new");
        assert_eq!(std::fs::read(&program).unwrap(), b"new");
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&program).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);
        // No staged files left behind.
        let leftovers: Vec<_> = std::fs::read_dir(link.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(leftovers, ["pairee"]);
        // A later update of the linked program replaces the file behind it.
        write_binary(&resolve_link(&link), b"newer").unwrap();
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(std::fs::read(&program).unwrap(), b"newer");
    }
}
