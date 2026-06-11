use super::context::AppContext;
use super::state::{AppState, PopupType};
use crate::terminal::{Event, EventHandler, TerminalBackend};
use crate::ui;
use anyhow::Result;
use std::time::Duration;

use super::actions::handle_action;
use super::input::handle_cli_input;
use super::input_popup::handle_popup_input;
use super::screen_input::handle_screen_input;

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

    // Launch background external tools download/check
    tokio::spawn(async {
        if let Err(e) = crate::fs::external_tools::ensure_external_tools().await {
            log::warn!("Failed to download external tools: {}", e);
        }
    });

    loop {
        // 1. Process background operation updates (e.g. copy progress)
        if state.progress_rx.is_some() {
            let mut rx = state.progress_rx.take().unwrap();
            let mut is_completed = false;
            let mut has_error = None;
            let mut latest_update = None;

            while let Ok(update) = rx.try_recv() {
                if let Some(err) = update.error.clone() {
                    has_error = Some(err);
                } else if update.current_file == "Completed" {
                    is_completed = true;
                } else {
                    latest_update = Some(update);
                }
            }

            if let Some(err) = has_error {
                state.active_popup = Some(PopupType::Error(err));
            } else if is_completed {
                state.active_popup = None;
                state.refresh_both_panels(context.config.settings.show_hidden);
            } else {
                if let Some(update) = latest_update {
                    let should_update = match &state.active_popup {
                        None | Some(PopupType::CopyProgress { .. }) => true,
                        _ => false,
                    };
                    if should_update {
                        state.active_popup = Some(PopupType::CopyProgress {
                            current_file: update.current_file,
                            files_copied: update.files_copied,
                            total_files: update.total_files,
                            bytes_copied: update.bytes_copied,
                            total_bytes: update.total_bytes,
                        });
                    }
                }
                state.progress_rx = Some(rx);
            }

            // 1.5 Process Terminal background updates
            if state.term_rx.is_some() {
                let mut rx = state.term_rx.take().unwrap();
                while let Ok(update) = rx.try_recv() {
                    if let Some(crate::app::state::Screen::Terminal(ts)) =
                        state.screens.get_mut(update.screen_idx)
                    {
                        match update.line {
                            Some(line) => ts.output_lines.push(line),
                            None => ts.is_running = false,
                        }
                    }
                }
                state.term_rx = Some(rx);
            }
        }

        // 2. Draw terminal window
        terminal_backend.terminal.draw(|f| {
            ui::draw_ui(f, &context, &state);
        })?;

        // 3. Exit check
        if state.should_quit {
            if context.config.settings.auto_save_setup {
                let _ = context.config.save();
            }
            // Save history store to disk
            let mut history_store = crate::config::history::HistoryStore::default();
            history_store.commands = state.command_history.clone();
            history_store.viewed_files = state.file_view_history.clone();
            history_store.visited_folders = state.folders_history.clone();
            let _ = history_store.save();
            break;
        }

        // 4. Handle input events
        if let Some(event) = event_handler.next().await {
            match event {
                Event::Key(key) => {
                    // Always track the most recent keyboard modifiers
                    state.current_modifiers = key.modifiers;

                    // Filter out KeyRelease events on Windows to prevent double-step triggers
                    if key.kind == crossterm::event::KeyEventKind::Release {
                        continue;
                    }

                    // Popups consume inputs first
                    match handle_popup_input(&mut state, key, &mut context) {
                        Ok(Some(action)) => {
                            handle_action(&mut state, action, &mut context, &mut terminal_backend)
                                .await?;
                            continue;
                        }
                        Ok(None) => {
                            continue;
                        }
                        Err(()) => {}
                    }

                    // Screens consume inputs before CLI and Panels (unless it's a global shortcut)
                    if handle_screen_input(&mut state, key, &mut context).is_ok() {
                        continue;
                    }

                    // CLI input takes priority next if applicable
                    if handle_cli_input(&mut state, key, &context, &mut terminal_backend).is_ok() {
                        continue;
                    }

                    // Standard resolved actions
                    if let Some(action) = context.resolver.resolve(key) {
                        handle_action(&mut state, action, &mut context, &mut terminal_backend)
                            .await?;
                    }
                }
                Event::ModifiersChanged(modifiers) => {
                    state.current_modifiers = modifiers;
                }
                Event::Resize(w, h) => {
                    log::debug!("Terminal resized to {}x{}", w, h);
                }
                Event::Tick => {}
                Event::Mouse(mouse) => {
                    log::debug!("Mouse event: {:?}", mouse);
                }
            }
        }
    }

    Ok(())
}
