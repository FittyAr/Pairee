use super::spec::{SourceKind, TabSpec};
use super::*;
use crate::app::state::{ActivePanel, AppState, PanelViewMode, SortField};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn tabs_at(paths: &[&str]) -> PanelTabs {
    let mut tabs = PanelTabs::new(PanelState::new(PathBuf::from(paths[0])));
    for path in &paths[1..] {
        tabs.insert_after(None, Tab::new(PanelState::new(PathBuf::from(path))));
    }
    tabs
}

fn paths(tabs: &PanelTabs) -> Vec<String> {
    tabs.tabs()
        .iter()
        .map(|t| t.panel.current_path.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn new_tabs_open_after_the_active_one_and_become_active() {
    let mut tabs = tabs_at(&["/a", "/b"]);
    tabs.activate_index(0);
    let id = tabs.insert_after(None, Tab::new(PanelState::new("/c".into())));
    assert_eq!(paths(&tabs), ["/a", "/c", "/b"]);
    assert_eq!(tabs.active().id, id);
    assert_eq!(tabs.active_index(), 1);
}

#[test]
fn the_last_tab_cannot_be_closed() {
    let mut tabs = tabs_at(&["/a"]);
    let id = tabs.active().id;
    assert!(!tabs.close(id));
    assert_eq!(tabs.count(), 1);
}

#[test]
fn closing_keeps_a_sensible_active_tab() {
    let mut tabs = tabs_at(&["/a", "/b", "/c"]);
    // Closing the active last tab activates its left neighbour.
    assert!(tabs.close(tabs.active().id));
    assert_eq!(paths(&tabs), ["/a", "/b"]);
    assert_eq!(tabs.active_index(), 1);
    // Closing a tab left of the active one keeps the same tab active.
    let b = tabs.active().id;
    let a = tabs.tabs()[0].id;
    assert!(tabs.close(a));
    assert_eq!(tabs.active().id, b);
}

#[test]
fn ids_are_stable_and_unique_across_moves() {
    let mut tabs = tabs_at(&["/a", "/b", "/c"]);
    let ids: Vec<TabId> = tabs.tabs().iter().map(|t| t.id).collect();
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        3
    );
    tabs.activate(ids[0]);
    assert!(tabs.move_active(true));
    assert_eq!(paths(&tabs), ["/b", "/a", "/c"]);
    assert_eq!(tabs.active().id, ids[0], "the moved tab stays active");
    assert_eq!(
        tabs.find(ids[2]).unwrap().panel.current_path,
        Path::new("/c")
    );
    assert!(tabs.move_active(false));
    assert!(!tabs.move_active(false), "no wrapping at the left edge");
}

#[test]
fn cycling_wraps_and_activation_reports_changes() {
    let mut tabs = tabs_at(&["/a", "/b", "/c"]);
    assert!(tabs.cycle(true));
    assert_eq!(tabs.active_index(), 0);
    assert!(tabs.cycle(false));
    assert_eq!(tabs.active_index(), 2);
    assert!(!tabs.activate_index(2), "already active");
    assert!(!tabs.activate_index(9), "out of range");
}

#[test]
fn spec_round_trips_through_toml() {
    let mut panel = PanelState::new("/data".into());
    panel.view_mode = PanelViewMode::Full;
    panel.sort_field = SortField::Size;
    panel.sort_reverse = true;
    panel.filter_mask = Some("*.rs".into());
    let mut tab = Tab::new(panel);
    tab.name = Some("work".into());
    tab.lock_here();
    let spec = tab.spec();
    assert_eq!(spec.source, SourceKind::Local);
    let text = toml::to_string(&spec).expect("serialize");
    let back: TabSpec = toml::from_str(&text).expect("deserialize");
    assert_eq!(back, spec);
    let restored = Tab::from_spec(&back);
    assert_ne!(restored.id, tab.id);
    assert_eq!(restored.spec(), spec);
}

#[test]
fn duplicate_is_an_unlocked_copy_of_the_location() {
    let mut tab = Tab::new(PanelState::new("/data/src".into()));
    tab.name = Some("pinned".into());
    tab.panel.cursor_index = 3;
    tab.lock_here();
    let copy = tab.duplicate();
    assert_ne!(copy.id, tab.id);
    assert!(copy.lock.is_none() && copy.name.is_none());
    assert_eq!(copy.panel.current_path, tab.panel.current_path);
    assert_eq!(copy.panel.cursor_index, 3);
    assert_eq!(copy.title(), "src");
    assert_eq!(tab.title(), "pinned");
}

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for sub in ["one", "two"] {
        std::fs::create_dir(dir.path().join(sub)).unwrap();
        std::fs::write(dir.path().join(sub).join(format!("{sub}.txt")), sub).unwrap();
    }
    dir
}

fn names(panel: &PanelState) -> Vec<String> {
    panel.entries.iter().map(|e| e.name.clone()).collect()
}

#[test]
fn locked_tab_opens_new_folders_in_a_new_tab() {
    let dir = tree();
    let mut state = AppState::new(dir.path().into(), dir.path().into());
    state.refresh_panel(ActivePanel::Left, false, true);
    state.toggle_active_tab_lock();
    let locked = state.panels.active_tab_id(ActivePanel::Left);
    state
        .get_active_panel_mut()
        .open_path(dir.path().join("one"));
    state.refresh_active_panel(false);
    let tabs = state.panels.tabs(ActivePanel::Left);
    assert_eq!(tabs.count(), 2);
    assert_ne!(tabs.active().id, locked);
    assert_eq!(names(tabs.panel()), ["..", "one.txt"]);
    let pinned = tabs.find(locked).unwrap();
    assert_eq!(pinned.panel.current_path, dir.path());
    assert!(names(&pinned.panel).contains(&"two".to_string()));
}

#[tokio::test]
async fn background_listing_lands_in_its_tab_after_a_switch() {
    let dir = tree();
    let mut state = AppState::new(dir.path().into(), dir.path().into());
    let first = state.panels.active_tab_id(ActivePanel::Left);
    state
        .get_active_panel_mut()
        .open_path(dir.path().join("one"));
    state.refresh_active_panel(false);
    // Switch away before the first tab's listing has been applied.
    state.duplicate_active_tab(false);
    state
        .get_active_panel_mut()
        .open_path(dir.path().join("two"));
    state.refresh_active_panel(false);
    let deadline = Instant::now() + Duration::from_secs(5);
    let loading = |s: &AppState| {
        s.panels
            .tabs(ActivePanel::Left)
            .tabs()
            .iter()
            .any(|t| t.panel.is_loading())
    };
    while loading(&state) && Instant::now() < deadline {
        state.poll_panel_listings();
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let tabs = state.panels.tabs(ActivePanel::Left);
    assert_ne!(tabs.active().id, first);
    assert_eq!(names(tabs.panel()), ["..", "two.txt"]);
    assert_eq!(names(&tabs.find(first).unwrap().panel), ["..", "one.txt"]);
}

#[test]
fn closing_a_tab_shows_its_neighbour_and_keeps_the_last() {
    let dir = tree();
    let mut state = AppState::new(dir.path().into(), dir.path().into());
    assert!(!state.close_active_tab(false), "a side keeps one tab");
    state.duplicate_active_tab(false);
    assert_eq!(state.panels.tabs(ActivePanel::Left).count(), 2);
    assert!(state.close_active_tab(false));
    assert_eq!(state.panels.tabs(ActivePanel::Left).count(), 1);
    assert_eq!(state.panels.tabs(ActivePanel::Right).count(), 1);
}
