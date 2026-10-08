//! Path jail for `pairee.fs`.
//!
//! The policy is built once in Rust when the runtime is bound and captured by
//! every fs closure, so a plugin cannot loosen it by mutating Lua globals.
//!
//! * Untrusted plugins: always jailed. Reads are allowed inside the plugin
//!   directory and the per-plugin data directory; writes only inside the data
//!   directory, and never inside Pairee's config directory.
//! * Trusted plugins in Secure Mode: jailed to workspace, config, cache,
//!   plugin and data directories.
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
    pub workspace: PathBuf,
}

impl FsPolicy {
    /// Build the policy for the plugin rooted at `plugin_dir`.
    pub fn new(plugin_dir: &Path, trusted: bool, secure_mode: bool) -> Self {
        let name = plugin_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());
        Self {
            trusted,
            secure_mode,
            plugin_dir: plugin_dir.to_path_buf(),
            data_dir: crate::config::paths::get_plugin_data_dir(&name),
            config_dir: crate::config::paths::get_config_dir(),
            cache_dir: crate::config::paths::get_cache_dir(),
            workspace: std::env::current_dir().unwrap_or_default(),
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
            self.workspace.join(&path)
        };
        let lexical = normalize_lexical(&absolute);
        let resolved = resolve_existing(&lexical)
            .ok_or_else(|| format!("Security violation: cannot resolve path {path:?}"))?;

        let roots: Vec<&PathBuf> = if self.trusted {
            vec![
                &self.plugin_dir,
                &self.data_dir,
                &self.workspace,
                &self.config_dir,
                &self.cache_dir,
            ]
        } else {
            match access {
                Access::Read => vec![&self.plugin_dir, &self.data_dir],
                Access::Write
                    if canonical_root(&self.config_dir)
                        .is_some_and(|cfg| resolved.starts_with(cfg)) =>
                {
                    Vec::new()
                }
                Access::Write => vec![&self.data_dir],
            }
        };
        let matched = roots
            .into_iter()
            .filter_map(|root| canonical_root(root))
            .find(|root| resolved.starts_with(root));

        if let Some(root) = matched {
            let rel = resolved
                .strip_prefix(&root)
                .map(Path::to_path_buf)
                .unwrap_or_default();
            Ok(Target::Jailed {
                root,
                rel,
                path: lexical,
            })
        } else {
            let mode = if self.trusted {
                "Secure Mode"
            } else {
                "the untrusted plugin sandbox"
            };
            Err(format!(
                "Security violation: path {path:?} is outside the directories permitted by {mode}"
            ))
        }
    }
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
