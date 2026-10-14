//! The last session (`<config_dir>/session.toml`): every tab of both
//! sides, the focused side and the panel layout, written on exit and read
//! at startup when `restore_session` is on.

use super::toml_store::StateFile;
use crate::app::state::ActivePanel;
use crate::app::state::tabs::spec::TabSpec;
use serde::{Deserialize, Serialize};

/// Tabs of one panel side.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideSession {
    /// Index of the tab shown on this side.
    #[serde(default)]
    pub active: usize,
    #[serde(default)]
    pub tabs: Vec<TabSpec>,
}

/// On-disk representation of `session.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionFile {
    pub active_side: ActivePanel,
    pub left_visible: bool,
    pub right_visible: bool,
    #[serde(default)]
    pub quick_view: bool,
    #[serde(default)]
    pub left: SideSession,
    #[serde(default)]
    pub right: SideSession,
}

impl SessionFile {
    pub fn side(&self, side: ActivePanel) -> &SideSession {
        match side {
            ActivePanel::Left => &self.left,
            ActivePanel::Right => &self.right,
        }
    }
}

/// Read with [`StateFile::read`]: a corrupted file is copied to
/// `session.toml.bak`, logged and ignored. With several instances running,
/// the last one to quit wins, but its session is written whole (under the
/// shared lock, see [`StateFile::save_to`]).
impl StateFile for SessionFile {
    const FILE_NAME: &'static str = "session.toml";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::tabs::spec::SourceKind;
    use crate::app::state::{PanelViewMode, SortField};
    use std::path::PathBuf;

    fn spec(path: &str, source: SourceKind) -> TabSpec {
        TabSpec {
            path: PathBuf::from(path),
            source,
            view_mode: PanelViewMode::Brief,
            sort_field: SortField::Size,
            sort_reverse: true,
            show_long_names: false,
            filter_mask: Some("*.rs".into()),
            name: Some("src".into()),
            locked: true,
            cursor: Some("main.rs".into()),
            connection: None,
            branch_view: true,
        }
    }

    fn sample() -> SessionFile {
        let mut remote = spec("/srv/www", SourceKind::Remote);
        remote.connection = Some("web".into());
        SessionFile {
            active_side: ActivePanel::Right,
            left_visible: true,
            right_visible: false,
            quick_view: true,
            left: SideSession {
                active: 1,
                tabs: vec![
                    spec("/home/u", SourceKind::Local),
                    spec("/home/u/a.zip/docs", SourceKind::Archive),
                ],
            },
            right: SideSession {
                active: 0,
                tabs: vec![remote],
            },
        }
    }

    #[test]
    fn session_round_trips_through_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SessionFile::FILE_NAME);
        let session = sample();
        session.save_to(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("connection = \"web\""));
        assert!(text.contains("branch_view = true"));
        assert!(!text.contains("password"));
        assert_eq!(SessionFile::read_from(&path), Some(session.clone()));

        // Sessions of older releases named the field `ssh_preset`.
        let old = text.replace("connection = \"web\"", "ssh_preset = \"web\"");
        std::fs::write(&path, old).unwrap();
        assert_eq!(SessionFile::read_from(&path), Some(session));
    }

    #[test]
    fn missing_session_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            SessionFile::read_from(&dir.path().join(SessionFile::FILE_NAME)),
            None
        );
    }

    #[test]
    fn corrupted_session_is_ignored_and_backed_up() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SessionFile::FILE_NAME);
        std::fs::write(&path, "active_side = [broken").unwrap();
        assert_eq!(SessionFile::read_from(&path), None);
        assert_eq!(
            std::fs::read_to_string(crate::config::load_guard::backup_path(&path)).unwrap(),
            "active_side = [broken"
        );
    }

    #[cfg(unix)]
    #[test]
    fn session_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(SessionFile::FILE_NAME);
        sample().save_to(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
