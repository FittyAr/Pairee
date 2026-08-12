pub mod background;
pub mod events;
pub mod updates;

use super::context::AppContext;
use super::state::AppState;
use crate::terminal::{EventHandler, TerminalBackend};
use crate::ui;
use anyhow::Result;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{QueueableCommand, execute};
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Runs the main loop for Pairee.
pub async fn run(mut context: AppContext, mut state: AppState) -> Result<()> {
    let mut terminal_backend = TerminalBackend::init()?;
    let mut event_handler = EventHandler::new(Duration::from_millis(50));

    // Load history store from disk
    let history_store = crate::config::history::HistoryStore::load();
    state.command_history = history_store.commands.clone();
    state.file_view_history = history_store.viewed_files.clone();
    state.folders_history = history_store.visited_folders.clone();

    // Initial folder scans
    state.refresh_both_panels(context.config.settings.show_hidden);
    state.mark_ui_dirty();

    // Launch background external tools download/check
    tokio::spawn(async {
        if let Err(e) = crate::fs::external_tools::ensure_external_tools().await {
            log::warn!("Failed to download external tools: {}", e);
        }
    });

    // Transfer progress redraw rate-limit (~12 Hz)
    const TRANSFER_DRAW_MIN: Duration = Duration::from_millis(80);

    loop {
        // 1. Process background operation updates (e.g. copy progress)
        let bg_before = state.ui_dirty;
        background::process_background_updates(&mut state, &context, &mut terminal_backend);
        // 1.8 Process self-update checking, progress tracking, and installer execution
        updates::process_update_events(&mut state, &mut context);
        // 1.9 Process plugin requests
        crate::plugin::process_plugin_requests(&mut state, &context);

        // Rate-limit transfer-driven redraws when only progress ticks changed.
        if state.transfer.is_some() {
            let active = state
                .transfer
                .as_ref()
                .map(|ts| ts.engine.queue.get_all().iter().any(|j| j.is_active()))
                .unwrap_or(false);
            if active {
                let now = Instant::now();
                let allow = state
                    .last_transfer_draw
                    .map(|t| now.duration_since(t) >= TRANSFER_DRAW_MIN)
                    .unwrap_or(true);
                if allow {
                    state.mark_ui_dirty();
                    state.last_transfer_draw = Some(now);
                } else if !bg_before {
                    // Don't leave ui_dirty stuck true every tick from other noise.
                }
            }
        }

        // 2. Draw only when dirty / needed (anti-glitch + less TTY load)
        if state.needs_redraw() {
            if state.terminal_needs_clear {
                let _ = terminal_backend.terminal.clear();
                state.terminal_needs_clear = false;
            }

            // DEC 2026 synchronized update: present the frame atomically when supported.
            let mut stdout = io::stdout();
            let _ = stdout.queue(BeginSynchronizedUpdate);
            let _ = stdout.flush();

            terminal_backend.terminal.draw(|f| {
                ui::draw_ui(f, &context, &state);
            })?;

            let _ = execute!(stdout, EndSynchronizedUpdate);
            state.ui_dirty = false;
        }

        // 3. Exit check
        if state.should_quit {
            if context.config.settings.auto_save_setup {
                context.config.save_logging();
            }
            // Save history store to disk
            let history_store = crate::config::history::HistoryStore {
                commands: state.command_history.clone(),
                viewed_files: state.file_view_history.clone(),
                visited_folders: state.folders_history.clone(),
            };
            let _ = history_store.save();
            break;
        }

        // 4. Handle input events
        if let Some(event) = event_handler.next().await {
            match &event {
                crate::terminal::Event::Key(_)
                | crate::terminal::Event::Mouse(_)
                | crate::terminal::Event::Resize(_, _)
                | crate::terminal::Event::ModifiersChanged(_) => {
                    state.mark_ui_dirty();
                }
                crate::terminal::Event::Tick => {}
            }
            events::handle_input_event(&mut state, &mut context, &mut terminal_backend, event)
                .await?;
        }
    }

    Ok(())
}
