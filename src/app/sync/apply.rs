//! Hands the reviewed synchronization plan to the Transfer Engine.

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::fs::sync::{SyncItem, plan_jobs};
use crate::fs::transfer::submit::submit_job;
use crate::fs::transfer::transfer_options_from_settings;

/// Queues one copy/delete job per group of actions (closing the dialog);
/// with nothing to do it says so instead.
pub fn apply_review(state: &mut AppState, context: &AppContext, items: &[SyncItem], mask: &str) {
    let jobs = plan_jobs(items);
    if jobs.is_empty() {
        state
            .dialogs
            .replace(PopupType::Info(t("sync_nothing_to_do")));
        return;
    }
    let base = transfer_options_from_settings(&context.config.settings);
    for planned in jobs {
        submit_job(state, planned.into_transfer_job(&base, mask));
    }
}
