//! Path jail for `pairee.fs`.
//!
//! The policy is built once in Rust when the runtime is bound and captured by
//! every fs closure, so a plugin cannot loosen it by mutating Lua globals.
//!
//! * Untrusted plugins: always jailed. Reads are allowed inside the plugin
//!   directory and the per-plugin data directory; writes only inside the data
//!   directory, and never inside Pairee's config directory.
//! * Trusted plugins in Secure Mode: reads are jailed to workspace, config,
//!   cache, plugin and data directories; writes to workspace, cache and data
//!   directories, never to the config directory outside the data directory.
//!   The workspace is the active panel's folder (see
//!   [`crate::plugin::workspace`]); the user's home or a filesystem root
//!   never counts as one.
//! * Trusted plugins otherwise: unrestricted (they already have `io`/`os`).
//!
//! Paths are normalized lexically and the nearest existing ancestor is
//! canonicalized, so `..` segments or symlinks inside non-existent tails
//! cannot escape the jail. A jailed check returns the canonical root it
//! matched plus the path relative to it; the operation itself then runs on a
//! capability handle of that root (see `capfs`), so swapping a component
//! for a symlink after the check cannot redirect it outside.

use super::target::Target;
use std::path::{Component, Path, PathBuf};

/// Kind of access a filesystem binding performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

/// Immutable sandbox policy captured by the fs closures.
#[derive(Clone, Debug)]
pub struct FsPolicy {
    pub trusted: bool,
    pub secure_mode: bool,
    pub plugin_dir: PathBuf,
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    /// An explicit workspace root. Empty: the active panel's folder, which
    /// changes as the user navigates (see [`crate::plugin::workspace`]).
    pub workspace: PathBuf,
}

impl FsPolicy {
    /// Build the policy for the plugin rooted at `plugin_dir`.
    pub fn new(plugin_dir: &Path, trusted: bool, secure_mode: bool) -> Self {
        let folder = plugin_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());
        let name = folder.strip_suffix(".pairee").unwrap_or(&folder);
        let data_dir = crate::config::paths::get_plugin_data_dir(name);
        if folder != name {
            migrate_legacy_data_dir(
                &crate::config::paths::get_plugin_data_dir(&folder),
                &data_dir,
            );
        }
        ensure_private_dir(&data_dir);
        Self {
            trusted,
            secure_mode,
            plugin_dir: plugin_dir.to_path_buf(),
            data_dir,
            config_dir: crate::config::paths::get_config_dir(),
            cache_dir: crate::config::paths::get_cache_dir(),
            workspace: PathBuf::new(),
        }
    }

    /// The workspace root now: the explicit one, else the active panel's
    /// folder; `None` when it is the home or a filesystem root.
    pub fn workspace_root(&self) -> Option<PathBuf> {
        if self.workspace.as_os_str().is_empty() {
            crate::plugin::workspace::current()
        } else {
            crate::plugin::workspace::usable(&self.workspace)
        }
    }

    /// Returns the target to operate on, or a security violation message.
    pub fn check(&self, path_str: &str, access: Access) -> Result<Target, String> {
        let path = PathBuf::from(path_str);
        if self.trusted && !self.secure_mode {
            return Ok(Target::Free(path));
        }

        let absolute = if path.is_absolute() {
            path.clone()
        } else {
            self.workspace_root()
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_default()
                .join(&path)
        };
        let lexical = normalize_lexical(&absolute);
        let resolved = resolve_existing(&lexical)
            .ok_or_else(|| format!("Security violation: cannot resolve path {path:?}"))?;

        let matched = self
            .roots(access, &resolved)
            .iter()
            .find_map(|root| jailed_in(root, &resolved, &lexical));
        if access == Access::Write
            && let Some(Target::Jailed { rel, .. }) = &matched
            && rel.components().any(|c| is_git_dir_name(c.as_os_str()))
        {
            return Err(format!(
                "Security violation: path {path:?} is inside a Git folder (.git), which plugins cannot write"
            ));
        }
        matched.ok_or_else(|| {
            let mode = if self.trusted {
                "Secure Mode"
            } else {
                "the untrusted plugin sandbox"
            };
            format!(
                "Security violation: path {path:?} is outside the directories permitted by {mode}"
            )
        })
    }

    /// Jail roots for `access` to the already resolved path `resolved`.
    ///
    /// Pairee's config directory (settings, keybindings, associations, user
    /// menu, plugins and `plugins.lock`) is read-only for every jailed
    /// plugin: a write there is only allowed inside the plugin's own data
    /// directory, so a plugin cannot switch Secure Mode off or trust itself.
    fn roots(&self, access: Access, resolved: &Path) -> Vec<PathBuf> {
        let workspace = self.workspace_root();
        let mut roots = vec![self.data_dir.clone()];
        match access {
            Access::Read if self.trusted => {
                roots.extend([self.plugin_dir.clone(), self.config_dir.clone()]);
                roots.extend(workspace);
                roots.push(self.cache_dir.clone());
            }
            Access::Read => roots.push(self.plugin_dir.clone()),
            Access::Write if self.inside_config(resolved) => {}
            Access::Write if self.trusted => {
                roots.extend(workspace);
                roots.push(self.cache_dir.clone());
            }
            Access::Write => {}
        }
        roots
    }

    fn inside_config(&self, resolved: &Path) -> bool {
        canonical_root(&self.config_dir).is_some_and(|cfg| resolved.starts_with(cfg))
    }
}

/// Creates the per-plugin data folder (owner-only on Unix) so a jail rooted
/// there can always be opened, even before the plugin writes anything.
fn ensure_private_dir(dir: &Path) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
}

/// Earlier versions kept the data in `<name>.pairee`; the data folder is now
/// the plain plugin name (as documented). Moves the old one over when the
/// new one does not exist yet.
fn migrate_legacy_data_dir(legacy: &Path, current: &Path) {
    if legacy.is_dir() && !current.exists() {
        let _ = std::fs::rename(legacy, current);
    }
}

/// Whether a path component names a Git folder: `.git` in any case, also
/// with the trailing dots / spaces or `:stream` suffix Windows ignores.
/// Its `config` and `hooks/` run programs whenever `git` is used there
/// (by a Secure Mode plugin's declared `git` or later by the user), so no
/// jailed plugin may write inside one.
fn is_git_dir_name(name: &std::ffi::OsStr) -> bool {
    let name = name.to_string_lossy();
    let name = name.split(':').next().unwrap_or_default();
    name.trim_end_matches(['.', ' '])
        .eq_ignore_ascii_case(".git")
}

/// `resolved` as a target under `root` when it lies inside it (`lexical`
/// is the path shown to Lua).
fn jailed_in(root: &Path, resolved: &Path, lexical: &Path) -> Option<Target> {
    let root = canonical_root(root)?;
    let rel = resolved.strip_prefix(&root).ok()?.to_path_buf();
    Some(Target::Jailed {
        root,
        rel,
        path: lexical.to_path_buf(),
    })
}

/// `path` (absolute) as a target jailed to `root`, or `None` when `..`
/// segments, symlinks or a dangling link would take it outside. Shared by
/// `pairee.fs` and the plugin `require` loader.
pub fn contain(root: &Path, path: &Path) -> Option<Target> {
    let lexical = normalize_lexical(path);
    let resolved = resolve_existing(&lexical)?;
    jailed_in(root, &resolved, &lexical)
}

/// Collapse `.` and `..` without touching the filesystem. `..` never climbs
/// above the root/prefix.
pub fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(_) | Component::RootDir => out.push(comp.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let at_root = matches!(
                    out.components().next_back(),
                    None | Some(Component::RootDir) | Some(Component::Prefix(_))
                );
                if !at_root {
                    out.pop();
                }
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

/// Canonicalize the nearest existing ancestor of an already lexically
/// normalized absolute path and re-append the missing tail. Returns `None`
/// for dangling symlinks, which could otherwise redirect a write outside.
pub fn resolve_existing(path: &Path) -> Option<PathBuf> {
    let mut existing = path.to_path_buf();
    let mut tail: Vec<std::ffi::OsString> = Vec::new();
    loop {
        if std::fs::symlink_metadata(&existing).is_ok() {
            let mut base = std::fs::canonicalize(&existing).ok()?;
            for part in tail.iter().rev() {
                base.push(part);
            }
            return Some(base);
        }
        let name = existing.file_name()?.to_os_string();
        tail.push(name);
        if !existing.pop() {
            return None;
        }
    }
}

/// `root` resolved the same way as checked paths.
fn canonical_root(root: &Path) -> Option<PathBuf> {
    resolve_existing(&normalize_lexical(root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct Fixture {
        _root: TempDir,
        policy: FsPolicy,
        outside: PathBuf,
    }

    fn fixture(trusted: bool, secure_mode: bool) -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(root.path()).unwrap();
        let config = base.join("config");
        let plugin = config.join("plugins").join("demo");
        let data = base.join("data").join("demo");
        let workspace = base.join("work");
        let outside = base.join("outside");
        for d in [&plugin, &data, &workspace, &outside] {
            std::fs::create_dir_all(d).unwrap();
        }
        let policy = FsPolicy {
            trusted,
            secure_mode,
            plugin_dir: plugin,
            data_dir: data,
            config_dir: config,
            cache_dir: base.join("cache"),
            workspace,
        };
        Fixture {
            _root: root,
            policy,
            outside,
        }
    }

    fn s(p: &Path) -> String {
        p.to_string_lossy().to_string()
    }

    #[test]
    fn lexical_normalization_collapses_dots() {
        let p = normalize_lexical(Path::new("/a/./b/../c/../../../d"));
        assert_eq!(p, PathBuf::from("/d"));
    }

    #[test]
    fn untrusted_is_jailed_even_without_secure_mode() {
        let f = fixture(false, false);
        let target = f.outside.join("x.txt");
        assert!(f.policy.check(&s(&target), Access::Read).is_err());
        assert!(f.policy.check(&s(&target), Access::Write).is_err());
    }

    #[test]
    fn untrusted_reads_plugin_dir_and_writes_data_dir() {
        let f = fixture(false, false);
        let script = f.policy.plugin_dir.join("main.lua");
        assert!(f.policy.check(&s(&script), Access::Read).is_ok());
        let new_file = f.policy.data_dir.join("sub").join("state.json");
        assert!(f.policy.check(&s(&new_file), Access::Write).is_ok());
        assert!(f.policy.check(&s(&new_file), Access::Read).is_ok());
    }

    #[test]
    fn untrusted_cannot_write_config_dir_or_own_code() {
        let f = fixture(false, false);
        let cfg = f.policy.config_dir.join("config.toml");
        assert!(f.policy.check(&s(&cfg), Access::Write).is_err());
        let script = f.policy.plugin_dir.join("main.lua");
        assert!(f.policy.check(&s(&script), Access::Write).is_err());
    }

    #[test]
    fn dotdot_on_missing_file_cannot_escape() {
        let f = fixture(false, false);
        let escape = format!("{}/missing/../../../outside/pwn.txt", s(&f.policy.data_dir));
        assert!(f.policy.check(&escape, Access::Write).is_err());
        let secure = fixture(true, true);
        let escape = format!("{}/nope/../../outside/pwn.txt", s(&secure.policy.workspace));
        assert!(secure.policy.check(&escape, Access::Write).is_err());
    }

    #[test]
    fn trusted_without_secure_mode_is_unrestricted() {
        let f = fixture(true, false);
        let target = s(&f.outside.join("x.txt"));
        assert_eq!(
            f.policy.check(&target, Access::Write).unwrap(),
            Target::Free(PathBuf::from(&target))
        );
    }

    #[test]
    fn secure_mode_config_dir_is_read_only() {
        let f = fixture(true, true);
        for name in [
            "config.toml",
            "keybindings.toml",
            "associations.toml",
            "plugins.lock",
            "plugins/other/main.lua",
        ] {
            let target = f.policy.config_dir.join(name);
            assert!(
                f.policy.check(&s(&target), Access::Write).is_err(),
                "{name}"
            );
            assert!(f.policy.check(&s(&target), Access::Read).is_ok(), "{name}");
        }
        let own_code = f.policy.plugin_dir.join("main.lua");
        assert!(f.policy.check(&s(&own_code), Access::Write).is_err());
        let data = f.policy.data_dir.join("state.json");
        assert!(f.policy.check(&s(&data), Access::Write).is_ok());
    }

    #[test]
    fn secure_mode_data_dir_inside_config_stays_writable() {
        let mut f = fixture(true, true);
        f.policy.data_dir = f.policy.config_dir.join("plugin-data").join("demo");
        std::fs::create_dir_all(&f.policy.data_dir).unwrap();
        let data = f.policy.data_dir.join("state.json");
        assert!(f.policy.check(&s(&data), Access::Write).is_ok());
        let cfg = f.policy.config_dir.join("config.toml");
        assert!(f.policy.check(&s(&cfg), Access::Write).is_err());
    }

    #[test]
    fn workspace_inside_config_dir_is_not_writable() {
        let mut f = fixture(true, true);
        f.policy.workspace = f.policy.config_dir.clone();
        let cfg = f.policy.config_dir.join("config.toml");
        assert!(f.policy.check(&s(&cfg), Access::Write).is_err());
    }

    #[test]
    fn git_folders_are_never_writable() {
        for (trusted, secure) in [(false, false), (true, true)] {
            let f = fixture(trusted, secure);
            let roots = if trusted {
                vec![&f.policy.workspace, &f.policy.data_dir]
            } else {
                vec![&f.policy.data_dir]
            };
            for root in roots {
                std::fs::create_dir_all(root.join("repo").join(".git")).unwrap();
                for rel in [
                    "repo/.git/config",
                    "repo/.git/hooks/pre-commit",
                    "repo/.GIT/config",
                    "repo/.git./config",
                    "new/.git",
                    "new/.git ",
                    "new/.git::$INDEX_ALLOCATION",
                ] {
                    let target = s(&root.join(rel));
                    let err = f.policy.check(&target, Access::Write).unwrap_err();
                    assert!(err.contains(".git"), "{rel}: {err}");
                }
                let config = s(&root.join("repo").join(".git").join("config"));
                assert!(f.policy.check(&config, Access::Read).is_ok() || !trusted);
                let fine = s(&root.join("repo").join(".gitignore"));
                assert!(f.policy.check(&fine, Access::Write).is_ok());
            }
        }
    }

    #[test]
    fn data_dir_is_the_plain_plugin_name_and_exists() {
        let policy = FsPolicy::new(Path::new("plugins/zz-data-demo.pairee"), false, false);
        assert_eq!(
            policy.data_dir,
            crate::config::paths::get_plugin_data_dir("zz-data-demo")
        );
        assert!(policy.data_dir.is_dir());
        let _ = std::fs::remove_dir(&policy.data_dir);
    }

    #[test]
    fn secure_mode_without_a_usable_workspace_only_reaches_its_own_folders() {
        let mut f = fixture(true, true);
        f.policy.workspace = PathBuf::new();
        crate::plugin::workspace::set_active(None);
        let work = f
            .policy
            .data_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("work");
        assert!(
            f.policy
                .check(&s(&work.join("a.txt")), Access::Write)
                .is_err()
        );
        let data = f.policy.data_dir.join("a.txt");
        assert!(f.policy.check(&s(&data), Access::Write).is_ok());
    }

    #[test]
    fn contain_keeps_paths_under_the_root() {
        let f = fixture(false, false);
        let root = &f.policy.plugin_dir;
        assert!(contain(root, &root.join("lib").join("util.lua")).is_some());
        assert!(contain(root, &root.join("..").join("..").join("x.lua")).is_none());
    }

    #[test]
    fn secure_mode_allows_workspace_and_denies_outside() {
        let f = fixture(true, true);
        let inside = f.policy.workspace.join("new.txt");
        assert!(f.policy.check(&s(&inside), Access::Write).is_ok());
        let outside = f.outside.join("x.txt");
        assert!(f.policy.check(&s(&outside), Access::Read).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_out_of_jail_is_rejected() {
        let f = fixture(false, false);
        let link = f.policy.data_dir.join("link");
        std::os::unix::fs::symlink(&f.outside, &link).unwrap();
        let through = link.join("pwn.txt");
        assert!(f.policy.check(&s(&through), Access::Write).is_err());
        let dangling = f.policy.data_dir.join("dangling");
        std::os::unix::fs::symlink(f.outside.join("nothing"), &dangling).unwrap();
        assert!(f.policy.check(&s(&dangling), Access::Write).is_err());
    }
}
