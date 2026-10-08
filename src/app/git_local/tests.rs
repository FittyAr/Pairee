//! Outside a Tokio runtime jobs run inline, so `run_git_local` applies the
//! result before returning; the `tokio` tests observe the busy phase.

use super::*;
use crate::app::context::AppContext;
use crate::app::input_popup::git_new_popups::{handle_confirm_action, open_confirm};
use crate::app::input_popup::git_panel::tabs::{handle_stash_tab, handle_status_tab};
use crate::app::state::GitConfirmedAction;
use crate::config::AppConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// A repository with one commit of `a.txt`.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    std::fs::write(dir.path().join("a.txt"), "a").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("a.txt")).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("T", "t@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "c1", &tree, &[])
        .unwrap();
    dir
}

/// State with the Git panel of `dir` open and loaded.
fn state_with_panel(dir: &Path) -> AppState {
    let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
    state.open_git_panel_at(dir, 100);
    state.poll_git_panel();
    state
}

fn panel(state: &AppState) -> &crate::app::state::GitPanelState {
    match state.dialogs.top() {
        Some(PopupType::GitPanel(panel)) => panel,
        other => panic!("expected the Git panel on top, got {other:?}"),
    }
}

fn top_error(state: &AppState) -> String {
    match state.dialogs.top() {
        Some(PopupType::Error(msg)) => msg.clone(),
        other => panic!("expected an error, got {other:?}"),
    }
}

#[test]
fn stage_runs_as_job_and_reloads_the_panel() {
    let dir = repo();
    std::fs::write(dir.path().join("b.txt"), "b").unwrap();
    let mut state = state_with_panel(dir.path());
    // The panel keys work on the path the snapshot reported (the canonical
    // work tree, e.g. `/private/var/...` on macOS or a long path instead of an
    // 8.3 temp path on Windows), exactly as `tab_action` passes it.
    let path = panel(&state).repo_path.clone();
    let entries = panel(&state).status_entries.clone();
    let idx = entries.iter().position(|e| e.path == "b.txt").unwrap();
    assert!(!entries[idx].is_staged);

    assert!(handle_status_tab(
        &mut state,
        KeyCode::Char(' '),
        &path,
        &entries,
        idx
    ));
    assert!(!state.git_panel.local.is_running(), "inline job finished");
    assert!(
        state.git_panel.is_loading(&path),
        "panel re-read after the job"
    );
    settle_panel(&mut state, &path);
    let staged = panel(&state)
        .status_entries
        .iter()
        .find(|e| e.path == "b.txt")
        .unwrap();
    assert!(staged.is_staged);
}

/// Applies Git panel results until the re-read of `path` finished (bounded).
fn settle_panel(state: &mut AppState, path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while state.git_panel.is_loading(path) {
        assert!(Instant::now() < deadline, "panel re-read never finished");
        if !state.poll_git_panel() {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn failure_opens_localized_alert_over_the_panel() {
    let dir = repo();
    let mut state = state_with_panel(dir.path());
    let stashes = [crate::git::stash::StashInfo {
        index: 7,
        message: "missing".into(),
        oid: "0".repeat(40),
    }];
    assert!(handle_stash_tab(
        &mut state,
        KeyCode::Char('a'),
        dir.path(),
        &stashes,
        0
    ));
    assert!(top_error(&state).starts_with(&t("git_error_stash_apply_failed")));
    state.dialogs.pop();
    assert!(matches!(state.dialogs.top(), Some(PopupType::GitPanel(_))));
}

#[test]
fn missing_repository_is_reported() {
    let dir = TempDir::new().unwrap();
    let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
    state.run_git_local(dir.path(), |_| Ok(()), |_, ()| panic!("must not run"));
    assert_eq!(top_error(&state), t("git_not_a_repo"));
}

#[test]
fn confirmed_action_runs_in_background_and_reports() {
    let dir = repo();
    let mut state = state_with_panel(dir.path());
    {
        let repo = git2::Repository::open(dir.path()).unwrap();
        crate::git::tags::create_tag(&repo, "v1", "HEAD", None).unwrap();
    }
    open_confirm(
        &mut state,
        dir.path(),
        "delete?".into(),
        GitConfirmedAction::DeleteTag("v1".into()),
    );
    let mut context = AppContext::new(AppConfig::default());
    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    handle_confirm_action(&mut state, enter, &mut context).unwrap();
    assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
    state.dialogs.pop();
    assert!(matches!(state.dialogs.top(), Some(PopupType::GitPanel(_))));
    let repo = git2::Repository::open(dir.path()).unwrap();
    assert!(repo.tag_names(None).unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn busy_while_running_and_refuses_a_second_operation() {
    let dir = repo();
    let mut state = state_with_panel(dir.path());
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    state.run_git_local(
        dir.path(),
        move |_| {
            let _ = release_rx.recv();
            Ok(())
        },
        |state, ()| state.dialogs.push(PopupType::Info("done".into())),
    );
    assert!(state.git_panel.is_working(dir.path()));
    state.run_git_local(dir.path(), |_| Ok(()), |_, ()| panic!("refused"));
    match state.dialogs.pop() {
        Some(PopupType::Info(msg)) => assert_eq!(msg, t("git_local_busy")),
        other => panic!("expected the busy notice, got {other:?}"),
    }

    release_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !poll_git_local(&mut state) {
        assert!(Instant::now() < deadline, "job never finished");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(!state.git_panel.is_working(dir.path()));
    assert!(matches!(state.dialogs.top(), Some(PopupType::Info(m)) if m == "done"));
}
