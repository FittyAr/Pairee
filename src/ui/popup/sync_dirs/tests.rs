//! TestBackend rendering of the Synchronize folders dialog.

use crate::app::context::AppContext;
use crate::app::state::popup::SyncDialog;
use crate::app::state::{AppState, PopupType};
use crate::config::AppConfig;
use crate::config::localization::t;
use crate::fs::sync::{SyncDirection, SyncOptions, diff_trees};
use crate::fs::transfer::options::HashAlgorithm;
use crate::ui::draw_ui;
use ratatui::{Terminal, backend::TestBackend};

struct NoCancel;

impl crate::fs::sync::ScanObserver for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn progress(&self, _: &crate::fs::sync::ScanProgress) {}
}

fn screen(state: &AppState) -> String {
    let context = AppContext::new(AppConfig::default());
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal.draw(|f| draw_ui(f, &context, state)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn state_with(dialog: SyncDialog) -> AppState {
    let mut state = AppState::new(dialog.left.clone(), dialog.right.clone());
    state.dialogs.replace(PopupType::SyncDirs(Box::new(dialog)));
    state
}

#[test]
fn options_form_shows_direction_toggles_and_buttons() {
    let options = SyncOptions {
        direction: SyncDirection::Mirror,
        ..SyncOptions::default()
    };
    let dialog = SyncDialog::new("L".into(), "R".into(), options, HashAlgorithm::Blake3);
    let text = screen(&state_with(dialog));
    assert!(text.contains(t("sync_direction_mirror").trim()));
    assert!(text.contains(t("sync_ignore_hidden").trim()));
    assert!(text.contains("BLAKE3"));
    assert!(text.contains(t("sync_btn_compare").trim()));
}

#[test]
fn review_lists_actions_summary_and_delete_confirmation() {
    let (l, r) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    std::fs::write(l.path().join("from_left.txt"), "abc").unwrap();
    std::fs::write(r.path().join("extra_right.txt"), "x").unwrap();
    let options = SyncOptions {
        direction: SyncDirection::Mirror,
        ..SyncOptions::default()
    };
    let items = diff_trees(l.path(), r.path(), &options, &NoCancel).unwrap();
    let mut dialog = SyncDialog::new(
        l.path().into(),
        r.path().into(),
        options,
        HashAlgorithm::Blake3,
    );
    dialog.show_review(items);
    let mut state = state_with(dialog);

    let text = screen(&state);
    assert!(text.contains("from_left.txt"));
    assert!(text.contains("extra_right.txt"));
    assert!(text.contains(t("sync_action_copy_right").trim()));
    assert!(text.contains(t("sync_action_delete_right").trim()));
    assert!(text.contains(t("sync_col_action").trim()));

    if let Some(PopupType::SyncDirs(dialog)) = state.dialogs.top_mut() {
        dialog.review.as_mut().unwrap().confirming = true;
    }
    assert!(screen(&state).contains(t("sync_confirm_title").trim()));
}

#[test]
fn empty_review_and_progress_popup_render() {
    let mut dialog = SyncDialog::new(
        "L".into(),
        "R".into(),
        SyncOptions::default(),
        HashAlgorithm::Blake3,
    );
    dialog.show_review(Vec::new());
    let mut state = state_with(dialog);
    assert!(screen(&state).contains("[Esc]"));

    state.dialogs.push(PopupType::FolderScanProgress);
    let counts = t("sync_scan_progress").replacen("{}", "0", 2);
    assert!(screen(&state).contains(counts.trim()));
}
