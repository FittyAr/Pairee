pub mod background;
pub mod events;
pub(crate) mod plugin_effects;
pub mod updates;

#[cfg(test)]
mod tests;

use super::context::AppContext;
use super::state::AppState;
use crate::app::state::popup::OnboardingKeymap;
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
    let mouse_probe = crate::terminal::mouse::EnvProbe::current();
    let mut terminal_backend =
        TerminalBackend::init(mouse_probe.captures(context.config.settings.mouse))?;
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
        // 0. Follow the mouse setting and `toggle_mouse_capture` live.
        let capture = mouse_probe.captures(context.config.settings.mouse) && !state.mouse_released;
        if let Err(err) = crate::terminal::mouse::set_capture(capture) {
            log::debug!("Mouse capture change failed: {err}");
        }

        // 1. Process background operation updates (e.g. copy progress)
        background::process_background_updates(&mut state, &context, &mut terminal_backend);
        // 1.8 Process self-update checking, progress tracking, and installer execution
        updates::process_update_events(&mut state, &mut context);
        // 1.9 Process plugin requests
        crate::plugin::process_plugin_requests(&mut state, &context);
        plugin_effects::apply(&mut state, &mut context, &mut terminal_backend).await?;

        // Rate-limit transfer-driven redraws when only progress ticks changed.
        if state
            .transfer
            .as_ref()
            .is_some_and(|ts| ts.engine.queue.has_active())
        {
            let now = Instant::now();
            let allow = state
                .redraw
                .last_transfer_draw
                .is_none_or(|t| now.duration_since(t) >= TRANSFER_DRAW_MIN);
            if allow {
                state.mark_ui_dirty();
                state.redraw.last_transfer_draw = Some(now);
            }
        }

        // 2. Draw only when dirty / needed (anti-glitch + less TTY load)
        if state.needs_redraw() {
            // DEC 2026 synchronized update: present clear+frame atomically when supported.
            let mut stdout = io::stdout();
            let _ = stdout.queue(BeginSynchronizedUpdate);
            let _ = stdout.flush();

            if state.redraw.needs_clear {
                let _ = terminal_backend.terminal.clear();
                state.redraw.needs_clear = false;
            }

            paint(&mut terminal_backend.terminal, &context, &state)?;

            let _ = execute!(stdout, EndSynchronizedUpdate);
            state.redraw.dirty = false;
        }

        // 3. Exit check
        if state.should_quit {
            return Ok(crate::app::session::persist_on_exit(
                &mut state,
                &mut context,
            ));
        }

        // 4. Handle input events (or wake early when a background job finished)
        let next_event = tokio::select! {
            ev = event_handler.next() => ev,
            _ = crate::app::jobs::finished() => None,
        };
        if let Some(event) = next_event {
            match &event {
                // Mouse events mark the UI dirty themselves when they
                // change something: pointer motion must not redraw.
                crate::terminal::Event::Key(_)
                | crate::terminal::Event::Resize(_, _)
                | crate::terminal::Event::ModifiersChanged(_)
                | crate::terminal::Event::Paste(_)
                | crate::terminal::Event::Terminate => {
                    state.mark_ui_dirty();
                }
                crate::terminal::Event::Mouse(_) | crate::terminal::Event::Tick => {}
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
    if context.config.settings.save_folders_history {
        state.frecency = crate::config::frecency::FrecencyStore::load();
    }

    // Initial folder scans
    state.refresh_both_panels(context.config.settings.show_hidden);
    if !context.config.settings.onboarding_completed {
        state
            .dialogs
            .replace(crate::app::state::PopupType::OnboardingKeymap(
                OnboardingKeymap { cursor_idx: 0 },
            ));
    }
    if let Some(msg) = context.config.take_load_error() {
        state.dialogs.push(crate::app::state::PopupType::error(msg));
    }
    state.mark_ui_dirty();
}

/// Draws one frame (which records its mouse targets in `state.hits`).
pub(crate) fn paint<B>(
    terminal: &mut ratatui::Terminal<B>,
    context: &AppContext,
    state: &AppState,
) -> Result<()>
where
    B: ratatui::backend::Backend,
    B::Error: Send + Sync + 'static,
{
    terminal.draw(|f| ui::draw_ui(f, context, state))?;
    Ok(())
}
