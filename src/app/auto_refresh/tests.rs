use super::*;
use crate::app::state::{ActivePanel, AppState};
use std::time::{Duration, Instant};

fn dirs(n: usize) -> Vec<tempfile::TempDir> {
    (0..n).map(|_| tempfile::tempdir().unwrap()).collect()
}

fn state_on(left: &Path, right: &Path) -> AppState {
    let mut state = AppState::new(left.to_path_buf(), right.to_path_buf());
    state.refresh_both_panels(false);
    state
}

fn monitored(state: &AppState) -> Vec<PathBuf> {
    state
        .auto_refresh
        .monitored()
        .into_iter()
        .map(Path::to_path_buf)
        .collect()
}

fn sorted(paths: &[&Path]) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = paths.iter().map(|p| p.to_path_buf()).collect();
    v.sort();
    v
}

fn settings(enabled: bool) -> Settings {
    Settings {
        auto_refresh: enabled,
        ..Settings::default()
    }
}

#[test]
fn monitors_the_folders_of_both_sides() {
    let d = dirs(2);
    let mut state = state_on(d[0].path(), d[1].path());
    state.poll_auto_refresh(false);
    assert_eq!(monitored(&state), sorted(&[d[0].path(), d[1].path()]));
}

#[test]
fn rearms_when_a_panel_changes_folder() {
    let d = dirs(3);
    let mut state = state_on(d[0].path(), d[1].path());
    state.poll_auto_refresh(false);
    state.panels.side_mut(ActivePanel::Left).current_path = d[2].path().to_path_buf();
    state.refresh_panel(ActivePanel::Left, false, false);
    state.poll_auto_refresh(false);
    assert_eq!(monitored(&state), sorted(&[d[1].path(), d[2].path()]));
}

#[test]
fn rearms_when_a_tab_closes() {
    let d = dirs(3);
    let mut state = state_on(d[0].path(), d[1].path());
    state.duplicate_active_tab(false);
    state.get_active_panel_mut().current_path = d[2].path().to_path_buf();
    state.refresh_active_panel(false);
    state.poll_auto_refresh(false);
    assert_eq!(monitored(&state), sorted(&[d[1].path(), d[2].path()]));
    assert!(state.close_active_tab(false));
    state.poll_auto_refresh(false);
    assert_eq!(monitored(&state), sorted(&[d[0].path(), d[1].path()]));
}

#[test]
fn disabled_monitors_nothing() {
    let d = dirs(2);
    let mut state = state_on(d[0].path(), d[1].path());
    state.poll_auto_refresh(false);
    state.auto_refresh.configure(&settings(false));
    state.poll_auto_refresh(false);
    assert!(monitored(&state).is_empty());
    state.auto_refresh.configure(&settings(true));
    state.poll_auto_refresh(false);
    assert_eq!(monitored(&state).len(), 2);
}

#[test]
fn large_folders_are_polled() {
    let d = dirs(2);
    for i in 0..3 {
        std::fs::write(d[0].path().join(format!("f{i}")), b"x").unwrap();
    }
    let mut state = state_on(d[0].path(), d[1].path());
    state.disable_panel_update_object_count = 2;
    let targets = state.auto_refresh_targets();
    assert!(targets[0].prefer_poll);
    assert!(!targets[1].prefer_poll);
}

#[test]
fn a_reported_change_rereads_the_tab_and_its_folder_sizes() {
    let d = dirs(2);
    let sub = d[0].path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    let mut state = state_on(d[0].path(), d[1].path());
    state.poll_auto_refresh(false);
    let panel = state.panels.side_mut(ActivePanel::Left);
    panel
        .dir_sizes
        .request(vec![sub.clone()], Default::default());
    panel.dir_sizes.poll();
    std::fs::write(sub.join("big"), [0u8; 10]).unwrap();
    std::fs::write(d[0].path().join("new"), b"x").unwrap();
    let sender = state.auto_refresh.sender();
    sender
        .send(DirChange {
            dir: d[0].path().to_path_buf(),
            entries: vec![sub.clone()],
        })
        .unwrap();
    let start = Instant::now();
    let panel_has = |state: &AppState| {
        state
            .panels
            .side(ActivePanel::Left)
            .entries
            .iter()
            .any(|e| e.name == "new")
    };
    while !panel_has(&state) && start.elapsed() < Duration::from_secs(10) {
        state.poll_auto_refresh(false);
        state.poll_panel_listings();
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(panel_has(&state));
    let panel = state.panels.side(ActivePanel::Left);
    assert!(!panel.is_loading(), "automatic refresh is silent");
    assert_eq!(panel.dir_sizes.get(&sub).unwrap().bytes, 10);
}

/// End to end with a real watcher: a file created in a shown folder ends
/// up in the panel without any explicit reread.
#[test]
fn new_file_reaches_the_panel() {
    let d = dirs(2);
    let mut state = state_on(d[0].path(), d[1].path());
    state.poll_auto_refresh(false);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut n = 0;
    let found = loop {
        // Keep creating files: the watcher may need a moment to arm.
        std::fs::write(d[0].path().join(format!("file{n}")), b"x").unwrap();
        n += 1;
        std::thread::sleep(Duration::from_millis(100));
        state.poll_auto_refresh(false);
        state.poll_panel_listings();
        let entries = &state.panels.side(ActivePanel::Left).entries;
        if entries.iter().any(|e| e.name.starts_with("file")) {
            break true;
        }
        if Instant::now() > deadline {
            break false;
        }
    };
    assert!(found, "no automatic refresh within 10 s");
}
