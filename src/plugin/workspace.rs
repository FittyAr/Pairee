//! The folder Secure Mode treats as the plugin workspace: the active
//! panel's local folder, refreshed by the main loop on every tick.
//!
//! The process launch folder is not a workspace: Pairee is often started
//! from `$HOME`, which would hand every Secure Mode plugin the dotfiles
//! (`.ssh`, `.bashrc`, ...). A folder that is the user's home or a
//! filesystem root is refused outright, so while the panel stands there
//! plugins have **no** workspace access (the plugin, data and cache folders
//! stay reachable).

use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// The folder last reported and the workspace it yields.
struct Active {
    reported: Option<PathBuf>,
    workspace: Option<PathBuf>,
}

static ACTIVE: RwLock<Active> = RwLock::new(Active {
    reported: None,
    workspace: None,
});

/// Records `dir` as the active panel's folder (`None`: a remote or virtual
/// panel, which has no local workspace). Cheap to call every tick.
pub fn set_active(dir: Option<&Path>) {
    if ACTIVE.read().is_ok_and(|a| a.reported.as_deref() == dir) {
        return;
    }
    if let Ok(mut slot) = ACTIVE.write() {
        slot.reported = dir.map(Path::to_path_buf);
        slot.workspace = dir.and_then(usable);
    }
}

/// The current workspace, if the active panel has a usable one.
pub fn current() -> Option<PathBuf> {
    ACTIVE.read().ok().and_then(|slot| slot.workspace.clone())
}

/// `dir` unless it is too broad to be a workspace.
pub fn usable(dir: &Path) -> Option<PathBuf> {
    (!too_broad(dir)).then(|| dir.to_path_buf())
}

/// `true` for the user's home folder and for a filesystem root (or a drive
/// root on Windows): a workspace there would expose far more than a project.
pub fn too_broad(dir: &Path) -> bool {
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if canonical.parent().is_none() {
        return true;
    }
    directories::BaseDirs::new().is_some_and(|dirs| {
        let home = dirs.home_dir();
        std::fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf()) == canonical
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_and_roots_are_not_workspaces() {
        let home = directories::BaseDirs::new()
            .map(|d| d.home_dir().to_path_buf())
            .unwrap();
        assert!(too_broad(&home));
        let root = std::env::current_dir()
            .unwrap()
            .ancestors()
            .last()
            .unwrap()
            .to_path_buf();
        assert!(too_broad(&root));
        assert!(usable(&root).is_none());
    }

    #[test]
    fn a_project_folder_is_a_workspace() {
        let project = tempfile::tempdir().unwrap();
        assert!(!too_broad(project.path()));
        assert_eq!(usable(project.path()).unwrap(), project.path());
    }
}
