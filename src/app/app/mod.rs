pub mod background;
pub mod events;
pub mod scrollbar_mouse;
pub mod tab_mouse;
pub mod updates;

use super::context::AppContext;
use super::state::AppState;
use crate::terminal::{EventHandler, TerminalBackend};
use crate::ui;
use anyhow::Result;
use crossterm::terminal::{BeginSynchronizedUpdate, EndSynchronizedUpdate};
use crossterm::{QueueableCommand, execute};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Runs the main loop for Pairee. Returns the local folder of the focused
/// panel at exit (for `--cwd-file` / `--print-cwd`).
pub async fn run(mut context: AppContext, mut state: AppState) -> Result<Option<PathBuf>> {
    let mut terminal_backend = TerminalBackend::init()?;
    let mut event_handler = EventHandler::new(Duration::from_millis(50));
    prepare_first_frame(&mut state, &mut context);

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
            // DEC 2026 synchronized update: present clear+frame atomically when supported.
            let mut stdout = io::stdout();
            let _ = stdout.queue(BeginSynchronizedUpdate);
            let _ = stdout.flush();

            if state.terminal_needs_clear {
                let _ = terminal_backend.terminal.clear();
                state.terminal_needs_clear = false;
            }

            paint(&mut terminal_backend.terminal, &context, &state)?;

            let _ = execute!(stdout, EndSynchronizedUpdate);
            state.ui_dirty = false;
        }

        // 3. Exit check
        if state.should_quit {
            return Ok(crate::app::session::persist_on_exit(&state, &mut context));
        }

        // 4. Handle input events (or wake early when a background job finished)
        let next_event = tokio::select! {
            ev = event_handler.next() => ev,
            _ = crate::app::jobs::finished() => None,
        };
        if let Some(event) = next_event {
            match &event {
                crate::terminal::Event::Key(_)
                | crate::terminal::Event::Mouse(_)
                | crate::terminal::Event::Resize(_, _)
                | crate::terminal::Event::ModifiersChanged(_)
                | crate::terminal::Event::Paste(_)
                | crate::terminal::Event::Terminate => {
                    state.mark_ui_dirty();
                }
                crate::terminal::Event::Tick => {}
            }
            events::handle_input_event(&mut state, &mut context, &mut terminal_backend, event)
                .await?;
        }
    }
}

/// Loads the history and folder shortcuts, scans both panels and queues the
/// start-up dialogs (onboarding, configuration load error).
pub(crate) fn prepare_first_frame(state: &mut AppState, context: &mut AppContext) {
    // Load history store from disk (only the categories the user chose to keep)
    state.history =
        crate::app::state::HistoryState::from_store(crate::app::session::persisted_history(
            crate::config::history::HistoryStore::load(),
            &context.config.settings,
        ));
    state.folder_shortcuts = crate::config::bookmarks::BookmarksFile::load().shortcut_map();

    // Initial folder scans
    state.refresh_both_panels(context.config.settings.show_hidden);
    if !context.config.settings.onboarding_completed {
        state
            .dialogs
            .replace(crate::app::state::PopupType::OnboardingKeymap { cursor_idx: 0 });
    }
    if let Some(msg) = context.config.take_load_error() {
        state.dialogs.push(crate::app::state::PopupType::Error(msg));
    }
    state.mark_ui_dirty();
}

/// Draws one frame, resetting the click targets the frame records.
pub(crate) fn paint<B>(
    terminal: &mut ratatui::Terminal<B>,
    context: &AppContext,
    state: &AppState,
) -> Result<()>
where
    B: ratatui::backend::Backend,
    B::Error: Send + Sync + 'static,
{
    state.scrollbar.clear_targets();
    state.tab_bar.clear();
    terminal.draw(|f| ui::draw_ui(f, context, state))?;
    Ok(())
}
