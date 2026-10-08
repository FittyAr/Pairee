//! Which external programs a plugin may spawn (`pairee.Command`,
//! `pairee.fs.spawn`).
//!
//! * Untrusted plugins can never spawn.
//! * Trusted plugins outside Secure Mode are unrestricted.
//! * Trusted plugins in Secure Mode use an **allowlist**: the program must be
//!   a bare name declared in the manifest (`[permissions] commands = [...]`),
//!   must not be a shell / interpreter / wrapper (hard deny list, also checked
//!   on the symlink target), and is resolved through absolute `PATH` entries.
//!   The resolved absolute path is what gets executed, so a renamed binary or
//!   an explicit path cannot slip through.

mod deny;
mod resolve;

pub use deny::normalize_command_name;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

/// Immutable spawn policy captured by the process closures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandPolicy {
    /// Untrusted plugin: no process may be started.
    Blocked,
    /// Trusted plugin outside Secure Mode.
    Unrestricted,
    /// Trusted plugin in Secure Mode: normalized names declared in the
    /// manifest.
    Allowlist(Vec<String>),
}

impl CommandPolicy {
    /// Build the policy from the plugin trust flag, Secure Mode and the
    /// commands declared in its manifest.
    pub fn new(trusted: bool, secure_mode: bool, declared: &[String]) -> Self {
        match (trusted, secure_mode) {
            (false, _) => Self::Blocked,
            (true, false) => Self::Unrestricted,
            (true, true) => {
                let mut names: Vec<String> = declared
                    .iter()
                    .filter(|d| is_bare_name(d))
                    .map(|d| normalize_command_name(d))
                    .filter(|n| !deny::is_denied(n))
                    .collect();
                names.sort();
                names.dedup();
                Self::Allowlist(names)
            }
        }
    }

    /// Returns the program to execute (an absolute path in Secure Mode) or
    /// a security violation message.
    pub fn authorize(&self, program: &str) -> Result<PathBuf, String> {
        let path_var = std::env::var_os("PATH").unwrap_or_default();
        self.authorize_with_path(program, &path_var)
    }

    fn authorize_with_path(&self, program: &str, path_var: &OsStr) -> Result<PathBuf, String> {
        let allowed = match self {
            Self::Blocked => {
                return Err(
                    "Security violation: spawning external processes is blocked in sandboxed mode."
                        .into(),
                );
            }
            Self::Unrestricted => return Ok(PathBuf::from(program)),
            Self::Allowlist(allowed) => allowed,
        };
        if !is_bare_name(program) {
            return Err(format!(
                "Security violation: Secure Mode only runs commands by name from PATH, not '{program}'"
            ));
        }
        let name = normalize_command_name(program);
        if deny::is_denied(&name) {
            return Err(format!(
                "Security violation: Command '{program}' is a shell, interpreter or wrapper and is never allowed in Secure Mode"
            ));
        }
        if !allowed.contains(&name) {
            return Err(format!(
                "Security violation: Command '{program}' is not declared in the plugin manifest ([permissions] commands)"
            ));
        }
        let resolved = resolve::resolve_in_path(program, path_var).ok_or_else(|| {
            format!("Security violation: Command '{program}' was not found in PATH")
        })?;
        let target = std::fs::canonicalize(&resolved).map_err(|e| {
            format!(
                "Security violation: cannot resolve '{}': {e}",
                resolved.display()
            )
        })?;
        let target_name = target
            .file_name()
            .map(|n| normalize_command_name(&n.to_string_lossy()))
            .unwrap_or_default();
        if deny::is_denied(&target_name) {
            return Err(format!(
                "Security violation: '{program}' resolves to '{}', which is never allowed in Secure Mode",
                target.display()
            ));
        }
        Ok(resolved)
    }
}

/// A single path component without directory, drive or `..`.
fn is_bare_name(program: &str) -> bool {
    !program.is_empty()
        && !program.contains(['/', '\\', ':'])
        && matches!(
            Path::new(program)
                .components()
                .collect::<Vec<_>>()
                .as_slice(),
            [Component::Normal(_)]
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[cfg(windows)]
    fn make_tool(dir: &Path, stem: &str) -> PathBuf {
        let p = dir.join(format!("{stem}.exe"));
        std::fs::write(&p, b"MZ").unwrap();
        p
    }

    #[cfg(not(windows))]
    fn make_tool(dir: &Path, stem: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let p = dir.join(stem);
        std::fs::write(&p, b"#!").unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }

    #[test]
    fn untrusted_can_never_spawn() {
        let p = CommandPolicy::new(false, false, &declared(&["git"]));
        assert_eq!(p, CommandPolicy::Blocked);
        assert!(p.authorize("git").unwrap_err().contains("sandboxed"));
    }

    #[test]
    fn trusted_without_secure_mode_is_unrestricted() {
        let p = CommandPolicy::new(true, false, &[]);
        assert_eq!(p.authorize("bash").unwrap(), PathBuf::from("bash"));
    }

    #[test]
    fn secure_mode_runs_declared_tool_by_absolute_path() {
        let dir = tempfile::tempdir().unwrap();
        let tool = make_tool(dir.path(), "mytool");
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let p = CommandPolicy::new(true, true, &declared(&["mytool"]));
        assert_eq!(p.authorize_with_path("mytool", &path_var).unwrap(), tool);
    }

    #[test]
    fn secure_mode_rejects_undeclared_tool() {
        let dir = tempfile::tempdir().unwrap();
        make_tool(dir.path(), "other");
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let p = CommandPolicy::new(true, true, &declared(&["mytool"]));
        let err = p.authorize_with_path("other", &path_var).unwrap_err();
        assert!(err.contains("not declared"), "{err}");
    }

    #[test]
    fn secure_mode_rejects_explicit_paths_even_if_declared() {
        let dir = tempfile::tempdir().unwrap();
        let tool = make_tool(dir.path(), "mytool");
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let p = CommandPolicy::new(true, true, &declared(&["mytool"]));
        for program in [
            tool.to_string_lossy().to_string(),
            "./mytool".to_string(),
            "..\\mytool".to_string(),
            "C:mytool".to_string(),
        ] {
            assert!(
                p.authorize_with_path(&program, &path_var).is_err(),
                "{program}"
            );
        }
    }

    #[test]
    fn secure_mode_never_allows_declared_shells() {
        let dir = tempfile::tempdir().unwrap();
        make_tool(dir.path(), "bash");
        make_tool(dir.path(), "python3");
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let p = CommandPolicy::new(true, true, &declared(&["bash", "python3", "cmd"]));
        assert_eq!(p, CommandPolicy::Allowlist(Vec::new()));
        for program in ["bash", "python3", "cmd.exe"] {
            let err = p.authorize_with_path(program, &path_var).unwrap_err();
            assert!(err.contains("never allowed"), "{err}");
        }
    }

    #[test]
    fn renamed_binary_outside_declaration_cannot_pass() {
        // A copy of a shell renamed to an innocuous name is only runnable
        // if the plugin declared that exact name; an undeclared one fails.
        let dir = tempfile::tempdir().unwrap();
        make_tool(dir.path(), "innocent");
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let p = CommandPolicy::new(true, true, &declared(&["git"]));
        assert!(p.authorize_with_path("innocent", &path_var).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn declared_name_symlinked_to_shell_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let bash = make_tool(dir.path(), "bash");
        std::os::unix::fs::symlink(&bash, dir.path().join("mytool")).unwrap();
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let p = CommandPolicy::new(true, true, &declared(&["mytool"]));
        let err = p.authorize_with_path("mytool", &path_var).unwrap_err();
        assert!(err.contains("resolves to"), "{err}");
    }

    #[test]
    fn declared_tool_missing_from_path_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path_var = std::env::join_paths([dir.path()]).unwrap();
        let p = CommandPolicy::new(true, true, &declared(&["mytool"]));
        let err = p.authorize_with_path("mytool", &path_var).unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }
}
