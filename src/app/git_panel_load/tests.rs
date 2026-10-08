//! Outside a Tokio runtime jobs run inline, but their results are only
//! applied by `poll_git_panel`, which lets these tests observe the
//! "loading" phase deterministically.

use super::*;
use std::path::PathBuf;
use tempfile::TempDir;

fn repo_with_commits(count: usize) -> TempDir {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    for i in 0..count {
        std::fs::write(dir.path().join("f.txt"), i.to_string()).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("f.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, &format!("c{i}"), &tree, &parents)
            .unwrap();
    }
    dir
}

fn new_state() -> AppState {
    AppState::new(PathBuf::from("."), PathBuf::from("."))
}

fn panel(state: &AppState) -> &GitPanelState {
    state
        .dialogs
        .iter()
        .find_map(|p| match p {
            PopupType::GitPanel(panel) => Some(panel),
            _ => None,
        })
        .expect("git panel open")
}

fn same_dir(a: &Path, b: &Path) -> bool {
    std::fs::canonicalize(a).unwrap() == std::fs::canonicalize(b).unwrap()
}

#[test]
fn panel_opens_empty_then_fills_in() {
    let dir = repo_with_commits(3);
    std::fs::write(dir.path().join("new.txt"), "x").unwrap();
    let mut state = new_state();
    state.open_git_panel_at(dir.path(), 100);
    assert!(panel(&state).log_entries.is_empty(), "nothing read yet");
    assert!(state.git_panel.is_loading(dir.path()));

    assert!(state.poll_git_panel());
    let p = panel(&state);
    assert!(!state.git_panel.is_loading(&p.repo_path));
    assert!(same_dir(&p.repo_path, dir.path()));
    assert_eq!(p.log_entries.len(), 3);
    assert!(p.log_complete);
    assert_eq!(p.status_entries.len(), 1);
    assert!(!p.current_branch.is_empty());
}

#[test]
fn outside_a_repository_shows_an_error() {
    let dir = TempDir::new().unwrap();
    let mut state = new_state();
    state.open_git_panel_at(dir.path(), 100);
    state.poll_git_panel();
    // A temp dir may sit inside some repository on a developer machine;
    // only assert the error when discovery really fails.
    if crate::git::repo::find_repo(dir.path()).is_none() {
        assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
    }
}

#[test]
fn refresh_keeps_contents_and_applies_only_the_latest_request() {
    let dir = repo_with_commits(1);
    let mut state = new_state();
    state.open_git_panel_at(dir.path(), 100);
    state.poll_git_panel();
    let repo = panel(&state).repo_path.clone();

    state.refresh_git_panel(&repo, 0, 5);
    std::fs::write(dir.path().join("late.txt"), "x").unwrap();
    state.refresh_git_panel(&repo, 0, 5);
    assert_eq!(
        panel(&state).log_entries.len(),
        1,
        "old contents stay visible"
    );

    state.poll_git_panel();
    let p = panel(&state);
    assert_eq!(p.status_entries.len(), 1, "result of the second request");
    assert_eq!(p.cursor_idx, 0, "cursor clamped to the list");
}

#[test]
fn result_reaches_a_panel_below_another_dialog() {
    let dir = repo_with_commits(2);
    let mut state = new_state();
    state.open_git_panel_at(dir.path(), 100);
    state.dialogs.push(PopupType::Info("done".into()));
    state.poll_git_panel();
    assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
    assert_eq!(panel(&state).log_entries.len(), 2);
}

#[test]
fn log_pages_load_in_the_background() {
    let dir = repo_with_commits(LOG_PAGE + 15);
    let mut state = new_state();
    state.open_git_panel_at(dir.path(), 10);
    state.poll_git_panel();
    assert!(!panel(&state).log_complete);

    if let Some(PopupType::GitPanel(p)) = state.dialogs.top_mut() {
        p.active_tab = 1;
        p.cursor_idx = 9;
    }
    state.prefetch_git_log();
    assert_eq!(panel(&state).log_entries.len(), 10, "page not applied yet");
    state.poll_git_panel();
    assert_eq!(panel(&state).log_entries.len(), 10 + LOG_PAGE);
    assert!(!panel(&state).log_complete);

    if let Some(PopupType::GitPanel(p)) = state.dialogs.top_mut() {
        p.cursor_idx = 10 + LOG_PAGE - 1;
    }
    state.prefetch_git_log();
    state.poll_git_panel();
    let p = panel(&state);
    assert_eq!(p.log_entries.len(), LOG_PAGE + 15);
    assert!(p.log_complete);
    assert_eq!(p.log_entries[0].message, format!("c{}", LOG_PAGE + 14));
}
