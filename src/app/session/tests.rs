use super::*;
use crate::app::state::tabs::spec::{SourceKind, TabSpec};
use crate::app::state::{ActivePanel, PanelViewMode, PopupType, SortField};
use crate::config::session::SideSession;
use crate::config::settings::SshPreset;
use std::path::Path;

fn spec(path: &Path, source: SourceKind) -> TabSpec {
    TabSpec {
        path: path.to_path_buf(),
        source,
        view_mode: PanelViewMode::Brief,
        sort_field: SortField::Date,
        sort_reverse: true,
        show_long_names: false,
        filter_mask: None,
        name: None,
        locked: false,
        cursor: Some("b.txt".into()),
        ssh_preset: None,
    }
}

fn remote_spec(preset: &str) -> TabSpec {
    TabSpec {
        ssh_preset: Some(preset.into()),
        ..spec(Path::new("/srv/www"), SourceKind::Remote)
    }
}

fn session(left: Vec<TabSpec>, right: Vec<TabSpec>) -> SessionFile {
    SessionFile {
        active_side: ActivePanel::Right,
        left_visible: true,
        right_visible: true,
        quick_view: true,
        left: SideSession {
            active: left.len().saturating_sub(1),
            tabs: left,
        },
        right: SideSession {
            active: 0,
            tabs: right,
        },
    }
}

fn settings_with_preset(name: &str) -> Settings {
    Settings {
        ssh_enabled: true,
        ssh_presets: vec![SshPreset {
            name: name.into(),
            host: "127.0.0.1".into(),
            port: "1".into(),
            username: "nobody".into(),
            password: None,
            key_path: None,
        }],
        ssh_timeout_secs: 1,
        ..Settings::default()
    }
}

fn new_state() -> AppState {
    AppState::new(".".into(), ".".into())
}

fn paths(state: &AppState, side: ActivePanel) -> Vec<PathBuf> {
    state
        .panels
        .tabs(side)
        .tabs()
        .iter()
        .map(|tab| tab.panel.current_path.clone())
        .collect()
}

#[test]
fn restores_tabs_layout_and_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = (dir.path().join("a"), dir.path().join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    let saved = session(
        vec![spec(&a, SourceKind::Local), spec(&b, SourceKind::Local)],
        vec![spec(&b, SourceKind::Local)],
    );
    let mut state = new_state();
    let notices = start_with(&mut state, &Settings::default(), Some(&saved), &[]);
    assert!(notices.is_empty());
    assert_eq!(paths(&state, ActivePanel::Left), [a.clone(), b.clone()]);
    assert_eq!(state.panels.tabs(ActivePanel::Left).active_index(), 1);
    assert_eq!(state.panels.active, ActivePanel::Right);
    assert!(state.panels.quick_view_active);
    let panel = state.panels.side(ActivePanel::Left);
    assert_eq!(panel.sort_field, SortField::Date);
    assert_eq!(panel.pending_focus.as_deref(), Some("b.txt"));

    // Capturing the restored state gives the same session back.
    assert_eq!(capture::capture(&state, &[]), saved);
}

#[test]
fn missing_folders_fall_back_to_the_nearest_ancestor() {
    let dir = tempfile::tempdir().unwrap();
    let gone = dir.path().join("x").join("y");
    let archive = dir.path().join("gone.zip").join("inner");
    let saved = session(
        vec![
            spec(&gone, SourceKind::Local),
            spec(&archive, SourceKind::Archive),
        ],
        vec![],
    );
    let mut state = new_state();
    start_with(&mut state, &Settings::default(), Some(&saved), &[]);
    let tabs = state.panels.tabs(ActivePanel::Left).tabs();
    for tab in tabs {
        assert_eq!(tab.panel.current_path, dir.path());
        assert!(tab.panel.pending_focus.is_none());
        assert!(matches!(
            tab.panel.source,
            crate::fs::vfs::PanelSource::Local
        ));
    }
    // A side saved without tabs keeps its default tab.
    assert_eq!(state.panels.tabs(ActivePanel::Right).count(), 1);
}

#[test]
fn existing_archive_tabs_keep_their_inner_path() {
    let dir = tempfile::tempdir().unwrap();
    let zip = dir.path().join("a.zip");
    std::fs::write(&zip, b"PK").unwrap();
    let inner = zip.join("docs");
    let saved = session(vec![spec(&inner, SourceKind::Archive)], vec![]);
    let mut state = new_state();
    start_with(&mut state, &Settings::default(), Some(&saved), &[]);
    assert_eq!(state.panels.side(ActivePanel::Left).current_path, inner);
}

#[test]
fn command_line_folders_override_the_shown_tabs() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b, c) = (
        dir.path().join("a"),
        dir.path().join("b"),
        dir.path().join("c"),
    );
    for d in [&a, &b, &c] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(c.join("f.txt"), b"x").unwrap();
    let saved = session(
        vec![spec(&a, SourceKind::Local), spec(&b, SourceKind::Local)],
        vec![remote_spec("web")],
    );
    let mut state = new_state();
    start_with(
        &mut state,
        &settings_with_preset("web"),
        Some(&saved),
        &[c.clone(), c.join("f.txt")],
    );
    // Only the shown tab of each side changes; the other tabs stay.
    assert_eq!(paths(&state, ActivePanel::Left), [a, c.clone()]);
    let right = state.panels.tabs(ActivePanel::Right).active();
    assert_eq!(right.panel.current_path, c);
    assert_eq!(right.panel.pending_focus.as_deref(), Some("f.txt"));
    assert!(right.pending_remote.is_none());
    // The view of the overridden tab is kept.
    assert_eq!(right.panel.view_mode, PanelViewMode::Brief);
}

#[test]
fn command_line_folders_without_a_session() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nope");
    let mut state = new_state();
    start_with(&mut state, &Settings::default(), None, &[missing]);
    assert_eq!(paths(&state, ActivePanel::Left), [dir.path().to_path_buf()]);
}

#[test]
fn ssh_tabs_wait_until_shown() {
    let saved = session(
        vec![remote_spec("web"), spec(Path::new("/"), SourceKind::Local)],
        vec![remote_spec("web")],
    );
    let settings = settings_with_preset("web");
    let mut state = new_state();
    let notices = start_with(&mut state, &settings, Some(&saved), &[]);
    assert!(notices.is_empty());
    let hidden = &state.panels.tabs(ActivePanel::Left).tabs()[0];
    let pending = hidden.pending_remote.clone().expect("pending");
    assert_eq!(pending.path, Path::new("/srv/www"));
    assert_eq!(hidden.panel.current_path, restore::home_dir());
    // The unvisited tab is saved again as the same SSH tab.
    assert_eq!(hidden.spec(), remote_spec("web"));

    // Only the shown tab (right side, focused) connects; the connection
    // fails and the tab stays local with a notice.
    let context = AppContext::new(crate::config::AppConfig {
        settings,
        ..Default::default()
    });
    remote::resume_pending(&mut state, &context);
    let result = state.ssh_connect.poll();
    let (id, result) = result.expect("the connection ran inline");
    assert_eq!(id, state.panels.active_tab_id(ActivePanel::Right));
    assert!(remote::finish(&mut state, id, &result, false));
    assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
    let right = state.panels.tabs(ActivePanel::Right).active();
    assert!(right.pending_remote.is_none());
    assert!(matches!(
        right.panel.source,
        crate::fs::vfs::PanelSource::Local
    ));
    // The hidden left tab is still waiting.
    let hidden = &state.panels.tabs(ActivePanel::Left).tabs()[0];
    assert!(
        hidden
            .pending_remote
            .as_ref()
            .is_some_and(|p| !p.connecting)
    );
}

#[test]
fn ssh_tabs_without_their_preset_open_at_home_with_a_notice() {
    let saved = session(vec![remote_spec("gone")], vec![]);
    let mut state = new_state();
    let notices = start_with(&mut state, &settings_with_preset("web"), Some(&saved), &[]);
    assert_eq!(notices.len(), 1);
    let tab = state.panels.tabs(ActivePanel::Left).active();
    assert!(tab.pending_remote.is_none());
    assert_eq!(tab.panel.current_path, restore::home_dir());
}
