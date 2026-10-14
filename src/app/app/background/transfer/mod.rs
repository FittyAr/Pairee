//! Transfer Engine background event processing.

mod events;

use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};

/// Log lines kept per transfer job (the oldest are dropped).
const MAX_JOB_LOG_LINES: usize = 1000;

pub fn process_transfer_events(state: &mut AppState, context: &AppContext) {
    let mut effects = events::EventEffects::default();

    if let Some(ref mut transfer_state) = state.transfer {
        let mut events = Vec::new();
        while let Ok(event) = transfer_state.event_rx.try_recv() {
            events.push(event);
        }

        for event in events {
            events::handle_transfer_event(event, state, context, &mut effects);
        }

        // Trimmed in place: copying every job (sources, results) to look at
        // its log stalled each pass of the event loop on big transfers.
        if let Some(ref transfer_state) = state.transfer {
            transfer_state.engine.queue.trim_logs(MAX_JOB_LOG_LINES);
        }
    }

    for (job_id, results) in effects.finished {
        let Some(job) = state
            .transfer
            .as_ref()
            .and_then(|ts| ts.engine.queue.get(job_id))
        else {
            continue;
        };
        // A failed or cancelled job reports the files it finished before.
        let results = results.unwrap_or_else(|| job.results.clone());
        crate::app::actions::fs_ops::undo::transfer_finished(state, &job, &results);
        if state.disk_usage.job_finished(&job, &results) | state.dupes.job_finished(&job, &results)
        {
            state.mark_ui_dirty();
        }
    }
    if !effects.term_forwards.is_empty() {
        for (job_id, line) in effects.term_forwards {
            for screen in state.screens.iter_mut() {
                if let Screen::Terminal(ts) = screen
                    && ts.job_id == Some(job_id)
                {
                    match &line {
                        Some(text) => ts.output.push(text.clone()),
                        None => ts.job_finished(),
                    }
                }
            }
        }
        state.mark_ui_dirty();
    }
    if effects.refresh_needed {
        state.refresh_both_panels(context.config.settings.show_hidden);
    }
    if effects.quit && state.may_quit() {
        state.should_quit = true;
    }
}
