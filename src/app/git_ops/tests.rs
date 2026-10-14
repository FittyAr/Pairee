use super::{FollowUp, GitNetOp};
use crate::app::state::popup::ErrorMessage;
use crate::app::state::popup::InfoMessage;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use std::path::PathBuf;

fn top_info(st: &AppState) -> Option<String> {
    match st.dialogs.top() {
        Some(PopupType::Info(InfoMessage(msg))) => Some(msg.clone()),
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
        Some(PopupType::Error(ErrorMessage(msg))) => {
            assert!(msg.starts_with(&t("git_error_fetch_failed")))
        }
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

/// Starts a fake operation whose job waits for `release`, then returns
/// `result` if it was cancelled meanwhile (as `run` would), else success.
fn start_gated(st: &mut AppState, result: super::GitOpResult) -> std::sync::mpsc::Sender<()> {
    let (release, gate) = std::sync::mpsc::channel::<()>();
    st.git_op.pending = Some((
        GitNetOp::Fetch {
            repo_path: PathBuf::from("."),
        },
        FollowUp::Info,
    ));
    st.dialogs.push(crate::app::state::popup::GitProgress {
        title: t("git_progress_pull"),
    });
    st.git_op.job.start(move |ctx| {
        let _ = gate.recv();
        if ctx.is_cancelled() {
            result
        } else {
            Ok(Vec::new())
        }
    });
    release
}

/// Polls until the operation's outcome was applied.
async fn finish(st: &mut AppState) {
    for _ in 0..500 {
        if super::poll_git_op(st) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("the Git operation never finished");
}

#[tokio::test]
async fn an_operation_done_despite_the_cancel_is_reported_as_done() {
    let mut st = state();
    let release = start_gated(&mut st, Ok(Vec::new()));
    st.cancel_git_op();
    assert!(st.git_op.is_running(), "the real outcome is awaited");
    match st.dialogs.top() {
        Some(PopupType::GitProgress(p)) => assert_eq!(p.title, t("git_operation_cancelling")),
        other => panic!("progress popup expected, got {other:?}"),
    }
    release.send(()).unwrap();
    finish(&mut st).await;
    assert_eq!(top_info(&st), Some(t("git_operation_done_before_cancel")));
    assert_eq!(st.dialogs.depth(), 1, "progress popup removed");
}

#[tokio::test]
async fn an_operation_stopped_by_the_cancel_is_reported_as_cancelled() {
    let mut st = state();
    let release = start_gated(&mut st, Err(None));
    st.cancel_git_op();
    // A second Esc hides the popup; the outcome still arrives.
    st.cancel_git_op();
    assert!(st.dialogs.top().is_none());
    release.send(()).unwrap();
    finish(&mut st).await;
    assert_eq!(top_info(&st), Some(t("git_operation_cancelled")));
}

#[test]
fn only_a_failure_after_the_cancel_counts_as_cancelled() {
    use super::run::outcome;
    assert_eq!(outcome(Ok(vec!["a".into()]), true), Ok(vec!["a".into()]));
    assert_eq!(outcome(Err(anyhow::anyhow!("x")), true), Err(None));
    assert_eq!(
        outcome(Err(anyhow::anyhow!("x")), false),
        Err(Some("x".into()))
    );
}

/// Commits `content` as `name` in the repository at `dir`.
fn commit_file(dir: &std::path::Path, name: &str, content: &str) {
    let repo = git2::Repository::open(dir).unwrap();
    std::fs::write(dir.join(name), content).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new(name)).unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let sig = git2::Signature::now("T", "t@example.com").unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, name, &tree, &[&parent])
        .unwrap();
}

#[test]
fn pull_with_conflicts_reports_the_files_instead_of_success() {
    let (work, bare) = repo_with_origin();
    let repo = git2::Repository::open(work.path()).unwrap();
    let branch = repo.head().unwrap().shorthand().unwrap().to_string();
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let push = |repo: &git2::Repository| {
        let mut remote = repo.find_remote("origin").unwrap();
        remote.push(&[refspec.as_str()], None).unwrap();
    };
    push(&repo);
    let other = tempfile::tempdir().unwrap();
    let url = bare.path().to_string_lossy().replace('\\', "/");
    let other_repo = git2::Repository::clone(&url, other.path()).unwrap();
    commit_file(other.path(), "a.txt", "theirs");
    push(&other_repo);
    commit_file(work.path(), "a.txt", "ours");

    let mut st = state();
    st.start_git_op(
        GitNetOp::Pull {
            repo_path: work.path().to_path_buf(),
            branch,
        },
        FollowUp::Info,
    );

    match st.dialogs.top() {
        Some(PopupType::Error(ErrorMessage(msg))) => {
            assert_eq!(*msg, t("git_pull_conflicts").replace("{}", "a.txt"))
        }
        other => panic!("expected the conflict report, got {other:?}"),
    }
    assert_eq!(repo.state(), git2::RepositoryState::Merge);
}

#[test]
fn conflict_summary_lists_the_first_files() {
    let paths: Vec<String> = (1..=7).map(|i| format!("f{i}")).collect();
    let summary = super::conflict_summary(&paths);
    assert!(summary.starts_with("f1, f2, f3, f4, f5 "), "{summary}");
    assert!(summary.ends_with(&t("git_conflicts_more").replace("{}", "2")));
    assert_eq!(super::conflict_summary(&paths[..1]), "f1");
}
