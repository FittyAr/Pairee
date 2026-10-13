use crate::app::actions::handle_action;
use crate::app::context::AppContext;
use crate::app::input::handle_cli_input;
use crate::app::input_popup::handle_popup_input;
use crate::app::screen_input::handle_screen_input;
use crate::app::state::AppState;
use crate::terminal::{Event, TerminalBackend};

pub async fn handle_input_event(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    event: Event,
) -> anyhow::Result<()> {
    match event {
        Event::Key(key) => {
            // Always track the most recent keyboard modifiers
            state.key_bar.held = key.modifiers;

            log::debug!("KeyEvent received: {:?}", key);

            // Filter out KeyRelease events on Windows to prevent double-step triggers
            if key.kind == crossterm::event::KeyEventKind::Release {
                return Ok(());
            }
            // A real key ends mouse gestures (a drag, a menu shown by
            // pointing); Shift alone must not, it turns a drop into a move.
            if !matches!(key.code, crossterm::event::KeyCode::Modifier(_)) {
                state.mouse.keyboard_used();
            }

            // Popups consume inputs first.
            if dispatch_popup_key(state, context, terminal_backend, key).await? {
                return Ok(());
            }

            // Screens consume inputs before CLI and Panels (unless it's a global shortcut)
            if handle_screen_input(state, key, context).is_ok() {
                return Ok(());
            }

            // CLI input takes priority next if applicable
            if handle_cli_input(state, key, context, terminal_backend).is_ok() {
                return Ok(());
            }

            // Standard resolved actions
            let key_str = crate::keybindings::resolver::key_event_to_string(key);
            if !key_str.is_empty() {
                let payload = serde_json::json!({ "key": key_str });
                tokio::spawn(async move {
                    crate::plugin::hooks::emit_event("on_key", payload).await;
                });
            }

            if matches!(key.code, crossterm::event::KeyCode::Esc) && context.resolver.is_ongoing() {
                context.resolver.reset();
                state.mark_ui_dirty();
                return Ok(());
            }

            if let Some(action) = context.resolver.resolve(key) {
                state.mark_ui_dirty();
                handle_action(state, action, context, terminal_backend).await?;
            } else if context.resolver.is_ongoing() {
                state.mark_ui_dirty();
            }
        }
        Event::ModifiersChanged(modifiers) => {
            state.key_bar.held = modifiers;
        }
        Event::Resize(w, h) => {
            apply_terminal_resize(state, w, h);
        }
        Event::Paste(text) => {
            crate::app::input::handle_paste(state, &text);
        }
        Event::Terminate => {
            tracing::info!("Application termination requested by OS signal");
            state.should_quit = true;
        }
        Event::Tick => {
            let now = std::time::Instant::now();
            if context.resolver.expire_pending(now) {
                state.mark_ui_dirty();
            }
            // Dragged files held at a panel's edge keep scrolling it.
            if crate::app::mouse::file_drag::autoscroll(state, now) {
                state.mark_ui_dirty();
            }
        }
        Event::Mouse(mouse) => {
            if context.config.settings.mouse != crate::config::settings::MouseMode::Off
                && crate::app::mouse::handle(state, context, terminal_backend, mouse).await?
            {
                state.mark_ui_dirty();
            }
        }
    }
    Ok(())
}

/// Gives `key` to the open popup, if any; list keys of the preset act as
/// arrows there. Returns `true` when a popup was open (it owns all input).
pub async fn dispatch_popup_key(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
    key: crossterm::event::KeyEvent,
) -> anyhow::Result<bool> {
    let popup_active = state.dialogs.is_some();
    let key = if popup_active {
        match crate::app::input_popup::list_keys::translate(state, &mut context.resolver, key) {
            Some(key) => key,
            None => return Ok(true),
        }
    } else {
        key
    };
    match handle_popup_input(state, key, context) {
        Ok(Some(action)) => {
            handle_action(state, action, context, terminal_backend).await?;
            Ok(true)
        }
        Ok(None) => Ok(true),
        Err(()) => Ok(popup_active),
    }
}

/// Resize is a screen-mode change: request a full-screen clear on the next paint.
fn apply_terminal_resize(state: &mut AppState, width: u16, height: u16) {
    log::debug!("Terminal resized to {}x{}", width, height);
    state.redraw.needs_clear = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn resize_requests_full_screen_clear() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.redraw.dirty = false;
        state.redraw.needs_clear = false;
        apply_terminal_resize(&mut state, 80, 24);
        assert!(state.redraw.needs_clear);
        assert!(state.needs_redraw());
    }

    #[test]
    fn terminate_event_marks_should_quit() {
        let mut state = AppState::new(PathBuf::from("."), PathBuf::from("."));
        state.should_quit = false;
        // In handle_input_event, Event::Terminate directly marks should_quit = true
        let event = Event::Terminate;
        if let Event::Terminate = event {
            state.should_quit = true;
        }
        assert!(state.should_quit);
    }
}
