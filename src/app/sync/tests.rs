//! Compare / synchronize flows on `AppState` (scan job, popups, apply).

use super::*;
use crate::config::AppConfig;
use crate::fs::CompareStatus;

fn context() -> AppContext {
    AppContext::new(AppConfig::default())
}

pub(crate) fn two_trees() -> (tempfile::TempDir, tempfile::TempDir) {
    let (l, r) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    std::fs::write(l.path().join("new.txt"), "new").unwrap();
    std::fs::create_dir(l.path().join("same")).unwrap();
    std::fs::create_dir(r.path().join("same")).unwrap();
    std::fs::write(r.path().join("extra.txt"), "extra").unwrap();
    (l, r)
}

/// Waits for the background scan (it runs on the blocking pool under Tokio).
pub(crate) async fn finish_scan(state: &mut AppState) {
    for _ in 0..500 {
        if poll_folder_scan(state) {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("folder scan did not finish");
}

#[test]
fn compare_runs_inline_and_lists_rolled_up_top_level_entries() {
    let (l, r) = two_trees();
    std::fs::write(l.path().join("same").join("inner.txt"), "x").unwrap();
    let mut state = AppState::new(l.path().to_path_buf(), r.path().to_path_buf());
    state.panels.left.entries.push(crate::fs::FileEntry {
        name: "new.txt".into(),
        path: l.path().join("new.txt"),
        size: 3,
        is_dir: false,
        is_symlink: false,
        modified: None,
    });

    start_compare(&mut state, &context());

    let Some(PopupType::CompareFoldersResult { diff, .. }) = state.dialogs.top() else {
        panic!("expected compare result, got {:?}", state.dialogs.top());
    };
    let status = |name: &str| diff.iter().find(|e| e.name == name).map(|e| &e.status);
    assert_eq!(status("new.txt"), Some(&CompareStatus::OnlyLeft));
    assert_eq!(status("extra.txt"), Some(&CompareStatus::OnlyRight));
    assert_eq!(
        status("same"),
        Some(&CompareStatus::Different),
        "folder differs because of what is inside"
    );
    assert!(
        state
            .panels
            .left
            .selected_paths
            .contains(&l.path().join("new.txt")),
        "differences are tagged in the left panel"
    );
    assert!(!state.folder_scan.is_running());
}

#[test]
fn missing_folder_reports_an_error() {
    let (l, r) = two_trees();
    let mut state = AppState::new(l.path().to_path_buf(), r.path().join("gone"));
    start_compare(&mut state, &context());
    assert!(matches!(state.dialogs.top(), Some(PopupType::Error(_))));
}

#[test]
fn open_dialog_starts_with_settings() {
    let (l, r) = two_trees();
    let mut state = AppState::new(l.path().to_path_buf(), r.path().to_path_buf());
    let mut context = context();
    context.config.settings.compare_mtime_tolerance_secs = 5;
    open_dialog(&mut state, &context);
    let Some(PopupType::SyncDirs(dialog)) = state.dialogs.top() else {
        panic!("sync dialog expected");
    };
    assert_eq!(dialog.left, l.path());
    assert_eq!(dialog.right, r.path());
    assert_eq!(
        dialog.options.compare.mtime_tolerance,
        std::time::Duration::from_secs(5)
    );
    assert!(dialog.review.is_none());
}

#[tokio::test]
async fn background_scan_reports_progress_and_can_be_cancelled() {
    let (l, r) = two_trees();
    let mut state = AppState::new(l.path().to_path_buf(), r.path().to_path_buf());
    start_compare(&mut state, &context());
    assert!(matches!(
        state.dialogs.top(),
        Some(PopupType::FolderScanProgress)
    ));
    assert_eq!(state.folder_scan.purpose(), Some(ScanPurpose::Compare));
    cancel_scan(&mut state);
    assert!(state.dialogs.top().is_none());
    assert!(!state.folder_scan.is_running());
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        !poll_folder_scan(&mut state),
        "a cancelled result is dropped"
    );

    start_compare(&mut state, &context());
    finish_scan(&mut state).await;
    assert!(matches!(
        state.dialogs.top(),
        Some(PopupType::CompareFoldersResult { .. })
    ));
}
