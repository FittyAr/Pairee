pub mod background;
pub mod events;
pub mod updates;

use super::context::AppContext;
use super::state::AppState;
use crate::terminal::{EventHandler, TerminalBackend};
use crate::ui;
use anyhow::Result;
use std::time::Duration;

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
        background::process_background_updates(&mut state, &context, &mut terminal_backend);

        // 1.8 Process self-update checking, progress tracking, and installer execution
        updates::process_update_events(&mut state, &mut context);

        // 1.9 Process plugin requests
        crate::plugin::process_plugin_requests(&mut state, &context);

        // 1.92 Auto-dismiss `PluginNotify` popups whose `deadline`
        //      has elapsed. The deadline is set by
        //      `dispatch_actions::render_notify` when a
        //      `pairee.notify({timeout=...})` is invoked.
        if let Some(crate::app::state::PopupType::PluginNotify { deadline: Some(d), .. }) =
            state.active_popup
        {
            if std::time::Instant::now() >= d {
                state.active_popup = None;
            }
        }

        // 1.95 Drain queued emit-actions (plugins called `pairee.emit(name, args)`)
        //      and execute them on the main thread with full access to state
        //      and the terminal backend.
        let pending = crate::plugin::drain_pending_emit_actions();
        for action in pending {
            if let Err(e) =
                crate::app::actions::handle_action(&mut state, action, &mut context, &mut terminal_backend)
                    .await
            {
                log::warn!("pairee.emit action dispatch error: {e}");
            }
        }

        // 2. Draw terminal window
        if state.terminal_needs_clear {
            let _ = terminal_backend.terminal.clear();
            state.terminal_needs_clear = false;
        }
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
            events::handle_input_event(&mut state, &mut context, &mut terminal_backend, event)
                .await?;
        }
    }

    Ok(())
}
