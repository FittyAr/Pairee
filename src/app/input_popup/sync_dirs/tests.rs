//! Driving the Synchronize folders dialog with keys, end to end.

use super::handle;
use crate::app::context::AppContext;
use crate::app::state::popup::SyncDialog;
use crate::app::state::{AppState, PopupType};
use crate::app::sync::tests::{finish_scan, two_trees};
use crate::config::AppConfig;
use crate::fs::sync::{SyncAction, SyncDirection};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn press(state: &mut AppState, context: &mut AppContext, code: KeyCode) {
    let key = KeyEvent::new(code, KeyModifiers::NONE);
    handle(state, key, context).unwrap();
}

fn dialog(state: &AppState) -> &SyncDialog {
    match state.dialogs.top() {
        Some(PopupType::SyncDirs(dialog)) => dialog,
        other => panic!("sync dialog expected, got {other:?}"),
    }
}

fn current_action(state: &mut AppState) -> SyncAction {
    let Some(PopupType::SyncDirs(dialog)) = state.dialogs.top_mut() else {
        panic!("sync dialog expected");
    };
    let review = dialog.review.as_mut().unwrap();
    review.current_mut().unwrap().action
}

/// Opens the dialog, picks Mirror and runs the comparison.
async fn reviewed_mirror(state: &mut AppState, context: &mut AppContext) {
    crate::app::sync::open_dialog(state, context);
    for _ in 0..3 {
        press(state, context, KeyCode::Char(' '));
    }
    assert_eq!(dialog(state).options.direction, SyncDirection::Mirror);
    press(state, context, KeyCode::Down);
    press(state, context, KeyCode::Char(' ')); // content comparison on
    assert!(dialog(state).options.content_hash.is_some());
    press(state, context, KeyCode::Down);
    press(state, context, KeyCode::Char(' ')); // ignore hidden
    press(state, context, KeyCode::Down);
    for c in "*.txt".chars() {
        press(state, context, KeyCode::Char(c));
    }
    press(state, context, KeyCode::Enter);
    finish_scan(state).await;
    assert_eq!(dialog(state).options.mask, "*.txt");
    assert!(dialog(state).review.is_some());
}

#[tokio::test]
async fn mirror_needs_confirmation_then_queues_engine_jobs() {
    let (l, r) = two_trees();
    let mut state = AppState::new(l.path().to_path_buf(), r.path().to_path_buf());
    let mut context = AppContext::new(AppConfig::default());
    reviewed_mirror(&mut state, &mut context).await;

    let review = dialog(&state).review.as_ref().unwrap();
    assert_eq!(review.visible().len(), 2, "folder pair hidden");
    let summary = review.summary();
    assert_eq!((summary.copy_right, summary.delete), (1, 1));

    press(&mut state, &mut context, KeyCode::Enter);
    assert!(dialog(&state).review.as_ref().unwrap().confirming);
    press(&mut state, &mut context, KeyCode::Char('n'));
    assert!(!dialog(&state).review.as_ref().unwrap().confirming);
    press(&mut state, &mut context, KeyCode::Enter);
    press(&mut state, &mut context, KeyCode::Char('y'));

    assert!(state.dialogs.top().is_none(), "dialog closed after apply");
    let jobs = state.transfer.as_ref().unwrap().engine.queue.get_all();
    assert_eq!(jobs.len(), 2, "one copy job and one delete job");
    assert!(
        jobs.iter()
            .all(|j| j.options.conflict_resolution == "overwrite")
    );
    assert_eq!(jobs[0].options.filter_mask.as_deref(), Some("*.txt;!.*"));
}

#[tokio::test]
async fn item_keys_change_actions_and_esc_goes_back() {
    let (l, r) = two_trees();
    let mut state = AppState::new(l.path().to_path_buf(), r.path().to_path_buf());
    let mut context = AppContext::new(AppConfig::default());
    reviewed_mirror(&mut state, &mut context).await;

    // Cursor on "extra.txt" (only on the right).
    assert_eq!(current_action(&mut state), SyncAction::DeleteRight);
    press(&mut state, &mut context, KeyCode::Left);
    assert_eq!(current_action(&mut state), SyncAction::CopyToLeft);
    press(&mut state, &mut context, KeyCode::Right);
    assert_eq!(
        current_action(&mut state),
        SyncAction::CopyToLeft,
        "not allowed"
    );
    press(&mut state, &mut context, KeyCode::Delete);
    assert_eq!(current_action(&mut state), SyncAction::DeleteRight);
    press(&mut state, &mut context, KeyCode::Char('s'));
    assert_eq!(current_action(&mut state), SyncAction::Skip);
    press(&mut state, &mut context, KeyCode::Char(' '));
    assert_eq!(current_action(&mut state), SyncAction::CopyToLeft);

    press(&mut state, &mut context, KeyCode::Tab);
    assert_eq!(dialog(&state).options.direction, SyncDirection::LeftToRight);
    assert_eq!(
        current_action(&mut state),
        SyncAction::Skip,
        "defaults follow"
    );

    press(&mut state, &mut context, KeyCode::Char('e'));
    assert!(dialog(&state).review.as_ref().unwrap().show_equal);

    press(&mut state, &mut context, KeyCode::Esc);
    assert!(dialog(&state).review.is_none(), "back to the options");
    press(&mut state, &mut context, KeyCode::Esc);
    assert!(state.dialogs.top().is_none());
}

#[tokio::test]
async fn nothing_to_apply_says_so() {
    let (l, r) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let mut state = AppState::new(l.path().to_path_buf(), r.path().to_path_buf());
    let mut context = AppContext::new(AppConfig::default());
    crate::app::sync::open_dialog(&mut state, &context);
    press(&mut state, &mut context, KeyCode::Up);
    press(&mut state, &mut context, KeyCode::Up);
    press(&mut state, &mut context, KeyCode::Enter);
    finish_scan(&mut state).await;
    press(&mut state, &mut context, KeyCode::Enter);
    assert!(matches!(state.dialogs.top(), Some(PopupType::Info(_))));
}
