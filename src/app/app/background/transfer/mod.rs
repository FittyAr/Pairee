//! Transfer Engine background event processing.

mod events;

use crate::app::context::AppContext;
use crate::app::state::{AppState, Screen};

pub fn process_transfer_events(state: &mut AppState, context: &AppContext) {
    let mut refresh_needed = false;
    let mut term_forwards: Vec<(uuid::Uuid, Option<String>)> = Vec::new();

    if let Some(ref mut transfer_state) = state.transfer {
        let mut events = Vec::new();
        while let Ok(event) = transfer_state.event_rx.try_recv() {
            events.push(event);
        }

        for event in events {
            events::handle_transfer_event(
                event,
                state,
                context,
                &mut term_forwards,
                &mut refresh_needed,
            );
        }

        if let Some(ref mut transfer_state) = state.transfer {
            let jobs = transfer_state.engine.queue.get_all();
            for job in jobs {
                if job.log_lines.len() > 1000 {
                    let job_id = job.id;
                    transfer_state.engine.queue.update_job(job_id, |j| {
                        let drain_count = j.log_lines.len() - 1000;
                        j.log_lines.drain(0..drain_count);
                    });
                }
            }
        }
    }

    if !term_forwards.is_empty() {
        for (job_id, line) in term_forwards {
            for screen in &mut state.screens {
                if let Screen::Terminal(ts) = screen
                    && ts.job_id == Some(job_id)
                {
                    match &line {
                        Some(text) => ts.output_lines.push(text.clone()),
                        None => ts.is_running = false,
                    }
                }
            }
        }
        state.mark_ui_dirty();
    }
    if refresh_needed {
        state.refresh_both_panels(context.config.settings.show_hidden);
    }
}
