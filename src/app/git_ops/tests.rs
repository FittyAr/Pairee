use super::{FollowUp, GitNetOp};
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use std::path::PathBuf;

fn top_info(st: &AppState) -> Option<String> {
    match st.dialogs.top() {
        Some(PopupType::Info(msg)) => Some(msg.clone()),
        _ => None,
    }
}

fn state() -> AppState {
    AppState::new(PathBuf::from("."), PathBuf::from("."))
}

/// A repository with one commit, plus a bare clone target as `origin`.
fn repo_with_origin() -> (tempfile::TempDir, tempfile::TempDir) {
    let work = tempfile::tempdir().unwrap();
    let bare = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(work.path()).unwrap();
    std::fs::write(work.path().join("a.txt"), "a").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("a.txt")).unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("T", "t@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "c1", &tree, &[])
        .unwrap();
    git2::Repository::init_bare(bare.path()).unwrap();
    let url = bare.path().to_string_lossy().replace('\\', "/");
    repo.remote("origin", &url).unwrap();
    (work, bare)
}

#[test]
fn fetch_without_remote_reports_error() {
    let work = tempfile::tempdir().unwrap();
    git2::Repository::init(work.path()).unwrap();
    let mut st = state();
    st.start_git_op(
        GitNetOp::Fetch {
            repo_path: work.path().to_path_buf(),
        },
        FollowUp::Info,
    );
    assert!(!st.git_op.is_running());
    match st.dialogs.top() {
        Some(PopupType::Error(msg)) => assert!(msg.starts_with(&t("git_error_fetch_failed"))),
        other => panic!("expected error, got {other:?}"),
    }
}

#[test]
fn fetch_success_shows_info_and_closes_progress() {
    let (work, _bare) = repo_with_origin();
    let mut st = state();
    st.start_git_op(
        GitNetOp::Fetch {
            repo_path: work.path().to_path_buf(),
        },
        FollowUp::Info,
    );
    assert_eq!(top_info(&st), Some(t("git_operation_success")));
    assert_eq!(st.dialogs.depth(), 1, "progress popup removed");
}

#[test]
fn clone_runs_and_refreshes() {
    let (work, _bare) = repo_with_origin();
    let dest = tempfile::tempdir().unwrap();
    let target = dest.path().join("copy");
    let mut st = state();
    st.start_git_op(
        GitNetOp::Clone {
            url: work.path().to_string_lossy().replace('\\', "/"),
            target: target.clone(),
        },
        FollowUp::Cloned { show_hidden: false },
    );
    assert!(target.join("a.txt").exists());
    assert_eq!(top_info(&st), Some(t("git_clone_success")));
}

#[tokio::test]
async fn cancel_drops_result_and_informs_user() {
    let (work, _bare) = repo_with_origin();
    let mut st = state();
    st.start_git_op(
        GitNetOp::Fetch {
            repo_path: work.path().to_path_buf(),
        },
        FollowUp::Info,
    );
    if st.git_op.is_running() {
        st.cancel_git_op();
        assert!(!st.git_op.is_running());
        assert_eq!(top_info(&st), Some(t("git_operation_cancelled")));
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        assert!(!super::poll_git_op(&mut st), "stale result ignored");
    }
}
