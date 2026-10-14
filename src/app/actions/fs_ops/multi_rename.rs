//! Multi-rename tool: opens the dialog for the targeted entries, runs the
//! planned renames in the background and reports the outcome.

use crate::app::context::AppContext;
use crate::app::state::popup::MultiRenameState;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::journal::{Direction, FsCommand};
use crate::fs::multi_rename::{RenameReport, RenameSource, Step, TargetFs, execute, plan};
use crate::fs::remote::{RemoteConn, endpoint_vfs};
use crate::fs::vfs::Vfs;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A finished batch of renames, the panel connection it ran on and the
/// undo/redo it ran for (`None`: the user's renames).
pub struct RenameRun {
    pub report: RenameReport,
    pub remote: Option<RemoteConn>,
    pub direction: Option<Direction>,
}

/// Opens the dialog for the selected entries (or the one under the cursor),
/// in listing order.
pub fn handle(state: &mut AppState) -> bool {
    let panel = state.get_active_panel();
    let targets: HashSet<PathBuf> = panel.get_targeted_paths().into_iter().collect();
    let sources: Vec<RenameSource> = panel
        .entries
        .iter()
        .filter(|entry| entry.name != ".." && targets.contains(&entry.path))
        .map(|entry| RenameSource {
            path: entry.path.clone(),
            is_dir: entry.is_dir,
            modified: entry.modified,
        })
        .collect();
    if sources.is_empty() {
        state
            .dialogs
            .replace(PopupType::error(t("error_no_entry_rename")));
        return true;
    }
    let siblings = panel
        .entries
        .iter()
        .filter(|entry| entry.name != "..")
        .map(|entry| entry.name.clone())
        .collect();
    let dialog = MultiRenameState::new(sources, siblings, panel.source.clone());
    state
        .dialogs
        .replace(PopupType::MultiRename(Box::new(dialog)));
    true
}

/// Starts the renames of the dialog on top, when its preview allows it.
pub fn start(state: &mut AppState) {
    let Some(PopupType::MultiRename(dialog)) = state.dialogs.top_mut() else {
        return;
    };
    if !dialog.can_run() {
        return;
    }
    dialog.running = true;
    let moves = dialog.preview.moves(&dialog.sources);
    let remote = dialog.source.remote().cloned();
    spawn(state, remote, None, move |backend, fs| {
        let taken = |path: &Path| backend.exists(path);
        match plan(&moves, fs, &taken) {
            Ok(steps) => execute(&steps, backend, fs),
            Err(duplicate) => RenameReport {
                failure: Some(crate::fs::multi_rename::RenameFailure {
                    error: t("multi_rename_issue_duplicate"),
                    path: duplicate.0,
                }),
                ..RenameReport::default()
            },
        }
    });
}

/// Runs already ordered `steps` (a rename on a server, or the undo/redo
/// `direction` of renames) in the background.
pub fn run_steps(
    state: &mut AppState,
    steps: Vec<Step>,
    remote: Option<RemoteConn>,
    direction: Option<Direction>,
) {
    spawn(state, remote, direction, move |backend, fs| {
        execute(&steps, backend, fs)
    });
}

/// Runs `job` on the panel's filesystem after the renames started before
/// it (renames are never interrupted: each batch is journaled).
fn spawn(
    state: &mut AppState,
    remote: Option<RemoteConn>,
    direction: Option<Direction>,
    job: impl FnOnce(&dyn Vfs, TargetFs) -> RenameReport + Send + 'static,
) {
    state.multi_rename.push(move || {
        let fs = TargetFs::for_panel(remote.is_some());
        let report = job(endpoint_vfs(&remote).as_ref(), fs);
        RenameRun {
            report,
            remote,
            direction,
        }
    });
}

/// Applies a finished rename job: closes the dialog or shows the error, and
/// rereads the panels either way.
pub fn poll(state: &mut AppState, context: &AppContext) {
    let Some(RenameRun {
        report,
        remote,
        direction,
    }) = state.multi_rename.poll()
    else {
        return;
    };
    let applied = FsCommand::Rename {
        steps: report.applied.clone(),
        remote,
    };
    state.journal.settle(direction, applied);
    if matches!(state.dialogs.top(), Some(PopupType::MultiRename(_))) {
        state.dialogs.pop();
    }
    if let Some(message) = failure_message(&report) {
        state.dialogs.replace(PopupType::error(message));
    } else {
        state.get_active_panel_mut().clear_selection();
    }
    state.refresh_both_panels(context.config.settings.show_hidden);
    state.mark_ui_dirty();
}

/// Error text of a failed batch, `None` on success.
fn failure_message(report: &RenameReport) -> Option<String> {
    let failure = report.failure.as_ref()?;
    let mut message = t("multi_rename_failed")
        .replacen("{}", &crate::fs::file_name_lossy(&failure.path), 1)
        .replacen("{}", &failure.error, 1);
    message.push('\n');
    if report.not_rolled_back.is_empty() {
        message.push_str(&t("multi_rename_rolled_back"));
    } else {
        let left: Vec<String> = report
            .not_rolled_back
            .iter()
            .map(|step: &Step| {
                format!(
                    "{} → {}",
                    crate::fs::file_name_lossy(&step.from),
                    crate::fs::file_name_lossy(&step.to)
                )
            })
            .collect();
        message.push_str(&t("multi_rename_rollback_incomplete").replacen(
            "{}",
            &left.join(", "),
            1,
        ));
    }
    Some(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::multi_rename::RenameFailure;

    #[tokio::test(flavor = "multi_thread")]
    async fn back_to_back_renames_are_all_applied_and_journaled_apart() {
        let dir = tempfile::tempdir().unwrap();
        let (a, b, c) = (
            dir.path().join("a"),
            dir.path().join("b"),
            dir.path().join("c"),
        );
        std::fs::write(&a, "x").unwrap();
        let mut state = AppState::new(dir.path().into(), dir.path().into());
        let context = AppContext::new(crate::config::AppConfig::default());
        let step = |from: &Path, to: &Path| Step {
            from: from.to_path_buf(),
            to: to.to_path_buf(),
        };
        run_steps(&mut state, vec![step(&a, &b)], None, None);
        run_steps(&mut state, vec![step(&b, &c)], None, Some(Direction::Undo));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while state.multi_rename.is_running() {
            assert!(std::time::Instant::now() < deadline, "renames lost");
            poll(&mut state, &context);
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(c.exists());
        assert_eq!(state.journal.len(Direction::Undo), 1, "the user's rename");
        assert_eq!(state.journal.len(Direction::Redo), 1, "the undo's");
    }

    #[test]
    fn success_has_no_message() {
        assert_eq!(failure_message(&RenameReport::default()), None);
    }

    #[test]
    fn failure_message_mentions_file_and_rollback() {
        let mut report = RenameReport {
            failure: Some(RenameFailure {
                path: PathBuf::from("a.txt"),
                error: "denied".into(),
            }),
            ..RenameReport::default()
        };
        let message = failure_message(&report).unwrap();
        assert!(message.contains("a.txt") && message.contains("denied"));
        assert!(message.contains(&t("multi_rename_rolled_back")));
        report.not_rolled_back.push(Step {
            from: PathBuf::from("b"),
            to: PathBuf::from("c"),
        });
        assert!(failure_message(&report).unwrap().contains("b → c"));
    }
}
