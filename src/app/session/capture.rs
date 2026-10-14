//! Describes the running session as a [`SessionFile`].

use crate::app::state::tabs::spec::{SourceKind, TabSpec};
use crate::app::state::tabs::{PanelTabs, Tab};
use crate::app::state::{ActivePanel, AppState};
use crate::config::session::{SessionFile, SideSession};
use crate::config::settings::Connection;
use crate::fs::remote::RemoteInfo;

/// The session to save: every tab of both sides plus the panel layout.
pub fn capture(state: &AppState, saved: &[Connection]) -> SessionFile {
    let panels = &state.panels;
    SessionFile {
        active_side: panels.active,
        left_visible: panels.left_visible,
        right_visible: panels.right_visible,
        quick_view: panels.quick_view_active,
        left: capture_side(panels.tabs(ActivePanel::Left), saved),
        right: capture_side(panels.tabs(ActivePanel::Right), saved),
    }
}

fn capture_side(tabs: &PanelTabs, saved: &[Connection]) -> SideSession {
    SideSession {
        active: tabs.active_index(),
        tabs: tabs.tabs().iter().map(|tab| tab_spec(tab, saved)).collect(),
    }
}

/// A tab's spec; a connected remote tab is saved by the name of the saved
/// connection it was opened with. Without a matching one it cannot be
/// reopened, so it is saved as the local home folder.
fn tab_spec(tab: &Tab, saved: &[Connection]) -> TabSpec {
    let mut spec = tab.spec();
    if spec.source != SourceKind::Remote || spec.connection.is_some() {
        return spec;
    }
    spec.connection = tab
        .panel
        .source
        .remote()
        .and_then(|conn| connection_for(conn.info(), saved));
    if spec.connection.is_none() {
        spec.source = SourceKind::Local;
        spec.path = super::restore::home_dir();
        spec.cursor = None;
        spec.branch_view = false;
    }
    spec
}

/// Name of the saved connection with a connection's kind, host, port and user.
fn connection_for(info: &RemoteInfo, saved: &[Connection]) -> Option<String> {
    saved
        .iter()
        .find(|c| c.matches(info.kind, &info.host, info.port, &info.username))
        .map(|c| c.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::settings::RemoteKind;

    fn saved(name: &str, kind: RemoteKind, host: &str, port: &str) -> Connection {
        Connection {
            name: name.into(),
            kind,
            host: host.into(),
            port: port.into(),
            username: "me".into(),
            password: Some(crate::credential_prompt::Secret::new("secret".into())),
            ..Connection::default()
        }
    }

    #[test]
    fn connections_map_to_their_saved_entry() {
        let list = [
            saved("a", RemoteKind::Sftp, "h1", "22"),
            saved("b", RemoteKind::Sftp, "h2", ""),
            saved("c", RemoteKind::Ftp, "h1", ""),
        ];
        let info = |kind, host: &str, port| RemoteInfo {
            kind,
            host: host.into(),
            port: Some(port),
            username: "me".into(),
        };
        let find = |i| connection_for(&i, &list);
        assert_eq!(find(info(RemoteKind::Sftp, "h1", 22)).as_deref(), Some("a"));
        assert_eq!(find(info(RemoteKind::Sftp, "h2", 22)).as_deref(), Some("b"));
        assert_eq!(find(info(RemoteKind::Sftp, "h1", 2222)), None);
        assert_eq!(find(info(RemoteKind::Ftp, "h1", 21)).as_deref(), Some("c"));
    }
}
