//! Describes the running session as a [`SessionFile`].

use crate::app::state::tabs::spec::{SourceKind, TabSpec};
use crate::app::state::tabs::{PanelTabs, Tab};
use crate::app::state::{ActivePanel, AppState};
use crate::config::session::{SessionFile, SideSession};
use crate::config::settings::SshPreset;

/// The session to save: every tab of both sides plus the panel layout.
pub fn capture(state: &AppState, presets: &[SshPreset]) -> SessionFile {
    let panels = &state.panels;
    SessionFile {
        active_side: panels.active,
        left_visible: panels.left_visible,
        right_visible: panels.right_visible,
        quick_view: panels.quick_view_active,
        left: capture_side(panels.tabs(ActivePanel::Left), presets),
        right: capture_side(panels.tabs(ActivePanel::Right), presets),
    }
}

fn capture_side(tabs: &PanelTabs, presets: &[SshPreset]) -> SideSession {
    SideSession {
        active: tabs.active_index(),
        tabs: tabs
            .tabs()
            .iter()
            .map(|tab| tab_spec(tab, presets))
            .collect(),
    }
}

/// A tab's spec; a connected SFTP tab is saved by the name of the preset it
/// was opened with. Without a matching preset it cannot be reopened, so it
/// is saved as the local home folder.
fn tab_spec(tab: &Tab, presets: &[SshPreset]) -> TabSpec {
    let mut spec = tab.spec();
    if spec.source != SourceKind::Remote || spec.ssh_preset.is_some() {
        return spec;
    }
    spec.ssh_preset = tab
        .panel
        .source
        .ssh()
        .and_then(|client| preset_for(client.info(), presets));
    if spec.ssh_preset.is_none() {
        spec.source = SourceKind::Local;
        spec.path = super::restore::home_dir();
        spec.cursor = None;
    }
    spec
}

/// Name of the preset matching a connection's host, port and user.
fn preset_for(info: &crate::fs::ssh::SshInfo, presets: &[SshPreset]) -> Option<String> {
    presets
        .iter()
        .find(|p| {
            p.host == info.host
                && p.username == info.username
                && p.port.parse::<u16>().unwrap_or(22) == info.port
        })
        .map(|p| p.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(name: &str, host: &str, port: &str) -> SshPreset {
        SshPreset {
            name: name.into(),
            host: host.into(),
            port: port.into(),
            username: "me".into(),
            password: Some("secret".into()),
            key_path: None,
        }
    }

    #[test]
    fn connections_map_to_their_preset() {
        let presets = [preset("a", "h1", "22"), preset("b", "h2", "")];
        let info = |host: &str, port| crate::fs::ssh::SshInfo {
            host: host.into(),
            port,
            username: "me".into(),
        };
        assert_eq!(preset_for(&info("h1", 22), &presets).as_deref(), Some("a"));
        assert_eq!(preset_for(&info("h2", 22), &presets).as_deref(), Some("b"));
        assert_eq!(preset_for(&info("h1", 2222), &presets), None);
    }
}
