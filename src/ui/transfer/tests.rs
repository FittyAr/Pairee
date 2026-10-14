//! The transfer bar, panel and conflict dialog, painted from the queue.
//! Jobs are put in the queue directly (no worker runs them): tests that
//! press keys or click while such a job "runs" tell the harness not to
//! wait for transfers.

use crate::app::state::{TransferTab, TransferViewMode};
use crate::config::localization::t;
use crate::fs::transfer::conflict::ConflictInfo;
use crate::fs::transfer::job::{
    TransferJob, TransferJobStatus, TransferOperation, TransferProgress,
};
use crate::fs::transfer::options::TransferOptions;
use crate::keybindings::Action;
use crate::test_harness::Harness;
use crate::ui::hit::HitTarget;

fn job(h: &Harness, operation: TransferOperation, name: &str) -> TransferJob {
    let source = h.left().join(name);
    TransferJob::new(
        operation,
        vec![source],
        h.right(),
        TransferOptions::default(),
    )
}

fn with_status(mut job: TransferJob, status: TransferJobStatus) -> TransferJob {
    job.status = status;
    job
}

/// A copy halfway through: `done` of `total` files, 45 % of the bytes.
fn running(h: &Harness, done: usize, total: usize) -> TransferJob {
    let mut job = with_status(
        job(h, TransferOperation::Copy, "a.txt"),
        TransferJobStatus::Transferring,
    );
    job.progress = Some(TransferProgress {
        current_file: h.left().join("a.txt").to_string_lossy().into_owned(),
        files_total: total,
        files_completed: done,
        bytes_total: 100,
        bytes_transferred: 45,
        ..Default::default()
    });
    job
}

/// Queues `jobs` (nothing runs them) and shows the transfers as `mode`.
fn queue(h: &mut Harness, jobs: Vec<TransferJob>, mode: TransferViewMode) {
    crate::app::transfer_submit::ensure_transfer_ui(&mut h.state);
    let ts = h.state.transfer.as_mut().expect("transfer state");
    for job in jobs {
        ts.engine.queue.enqueue(job);
    }
    ts.view_mode = mode;
}

fn bar_hit(h: &mut Harness) -> bool {
    h.render();
    let expand = HitTarget::Action(Action::ToggleTransferPanel);
    h.state
        .hits
        .first(|t| (*t == expand).then_some(()))
        .is_some()
}

#[test]
fn the_minimized_bar_shows_the_running_job_while_it_runs() {
    let mut h = Harness::new();
    let job = running(&h, 3, 10);
    let id = job.id;
    queue(&mut h, vec![job], TransferViewMode::Minimized);
    let screen = h.screen_text();
    assert!(screen.contains(&t("transfer_title")), "{screen}");
    assert!(screen.contains("3/10"), "{screen}");
    assert!(screen.contains("45%"), "{screen}");
    assert!(bar_hit(&mut h), "a click on the bar expands the panel");

    // Done: the bar (and its click target) goes away.
    let ts = h.state.transfer.as_ref().unwrap();
    ts.engine
        .queue
        .update_job(id, |j| j.status = TransferJobStatus::Completed);
    assert!(!bar_hit(&mut h));
    assert!(!h.screen_text().contains("3/10"));
}

#[test]
fn the_minimized_bar_expands_on_click_and_minimizes_again_while_a_job_runs() {
    let mut h = Harness::new();
    h.without_waiting_for_transfers();
    let job = running(&h, 3, 10);
    queue(&mut h, vec![job], TransferViewMode::Minimized);
    let (x, y) = h.hit_cell(|t| *t == HitTarget::Action(Action::ToggleTransferPanel));

    h.click(x, y);

    let ts = h.state.transfer.as_ref().unwrap();
    assert_eq!(ts.view_mode, TransferViewMode::Expanded);
    assert!(matches!(
        h.state.dialogs.top(),
        Some(crate::app::state::PopupType::TransferPanel(_))
    ));
    // Esc puts the panel back into the bar; the job keeps running.
    h.keys("Esc");
    assert_eq!(
        h.state.transfer.as_ref().unwrap().view_mode,
        TransferViewMode::Minimized
    );
    assert!(h.state.dialogs.top().is_none());
    assert!(bar_hit(&mut h), "the bar is back");
    // The toggle key opens and closes it as well.
    h.trigger("toggle_transfer_panel");
    assert_eq!(
        h.state.transfer.as_ref().unwrap().view_mode,
        TransferViewMode::Expanded
    );
    h.trigger("toggle_transfer_panel");
    assert_eq!(
        h.state.transfer.as_ref().unwrap().view_mode,
        TransferViewMode::Minimized
    );
}

#[test]
fn the_panels_keep_working_under_the_minimized_bar() {
    let mut h = Harness::new();
    h.write("work/left/a.txt", "a");
    h.write("work/left/b.txt", "b");
    h.reread();
    h.without_waiting_for_transfers();
    let job = running(&h, 1, 4);
    queue(&mut h, vec![job], TransferViewMode::Minimized);
    let before = h.state.get_active_panel().cursor_index;

    h.keys("Down");

    assert_eq!(h.state.get_active_panel().cursor_index, before + 1);
    assert!(h.screen_text().contains("1/4"), "the bar stays shown");
}

#[test]
fn waiting_for_transfers_ends_once_the_jobs_are_done() {
    let mut h = Harness::new();
    h.without_waiting_for_transfers();
    let job = running(&h, 1, 1);
    let id = job.id;
    queue(&mut h, vec![job], TransferViewMode::Minimized);
    let ts = h.state.transfer.as_ref().unwrap();
    ts.engine
        .queue
        .update_job(id, |j| j.status = TransferJobStatus::Completed);
    h.wait_for_transfers();
    assert!(!bar_hit(&mut h));
}

#[test]
fn the_expanded_panel_lists_jobs_and_inspects_the_selected_one() {
    let mut h = Harness::new();
    let copy = with_status(
        job(&h, TransferOperation::Copy, "a.txt"),
        TransferJobStatus::Completed,
    );
    let mut moved = with_status(
        job(&h, TransferOperation::Move, "b.txt"),
        TransferJobStatus::Failed,
    );
    moved.log_lines.push("cannot open b.txt".into());
    queue(&mut h, vec![copy, moved], TransferViewMode::Expanded);
    let screen = h.screen_text();
    assert!(screen.contains("#1 Copy - Completed"), "{screen}");
    assert!(screen.contains("#2 Move - Failed"), "{screen}");
    assert!(screen.contains("Dest: right"), "{screen}");
    assert!(screen.contains("Job status: Completed"), "{screen}");

    // The inspector follows the queue cursor and the selected tab.
    let ts = h.state.transfer.as_mut().unwrap();
    ts.queue_cursor = 1;
    ts.active_tab = TransferTab::Log;
    let screen = h.screen_text();
    assert!(screen.contains("Job status: Failed"), "{screen}");
    assert!(screen.contains("cannot open b.txt"), "{screen}");
}

#[test]
fn an_empty_queue_says_how_to_start_a_transfer() {
    let mut h = Harness::new();
    queue(&mut h, Vec::new(), TransferViewMode::Expanded);
    h.assert_screen("No jobs in queue.");
    // Hidden: nothing of it is painted.
    h.state.transfer.as_mut().unwrap().view_mode = TransferViewMode::Hidden;
    h.assert_no_screen("No jobs in queue.");
}

#[test]
fn a_conflict_names_the_file_and_offers_the_answers() {
    let mut h = Harness::new();
    let job = running(&h, 0, 1);
    let id = job.id;
    queue(&mut h, vec![job], TransferViewMode::Expanded);
    let (src, dst) = (h.left().join("a.txt"), h.right().join("a.txt"));
    let conflict = ConflictInfo {
        src_path: src,
        dst_path: dst.clone(),
        src_size: 2,
        dst_size: 1,
        src_modified: None,
        dst_modified: None,
    };
    h.state.transfer.as_mut().unwrap().active_conflict_info = Some((id, dst, conflict));
    let screen = h.screen_text();
    assert!(screen.contains("Conflict Detected"), "{screen}");
    assert!(screen.contains("a.txt"), "{screen}");
    assert!(screen.contains("[o] Overwrite"), "{screen}");
    assert!(screen.contains("[x] Cancel job"), "{screen}");
}

#[test]
fn summarized_paths_keep_the_folder_and_the_name() {
    use crate::ui::transfer::panel::summarize_path;
    let sep = std::path::MAIN_SEPARATOR;
    let path = std::path::Path::new("deep").join("folder").join("file.txt");
    assert_eq!(summarize_path(&path), format!("..{sep}folder{sep}file.txt"));
    assert_eq!(summarize_path(std::path::Path::new("file.txt")), "file.txt");
}
