pub mod background;
pub mod events;
pub mod scrollbar_mouse;
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

    // Load history store from disk (only the categories the user chose to keep)
    state.history = crate::app::state::HistoryState::from_store(persisted_history(
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

            state.scrollbar.clear_targets();
            terminal_backend.terminal.draw(|f| {
                ui::draw_ui(f, &context, &state);
            })?;

            let _ = execute!(stdout, EndSynchronizedUpdate);
            state.ui_dirty = false;
        }

        // 3. Exit check
        if state.should_quit {
            if context.config.settings.auto_save_setup {
                crate::app::sys_helpers::capture_setup(&state, &mut context.config.settings);
                context.config.save_logging();
            }
            // Save history store to disk, honoring the save_*_history settings
            let _ = persisted_history(state.history.to_store(), &context.config.settings).save();
            break;
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

    Ok(())
}

/// Drops the history categories whose `save_*_history` setting is disabled.
fn persisted_history(
    store: crate::config::history::HistoryStore,
    settings: &crate::config::settings::Settings,
) -> crate::config::history::HistoryStore {
    store.retain_enabled(
        settings.save_commands_history,
        settings.save_folders_history,
        settings.save_view_and_edit_history,
    )
}
