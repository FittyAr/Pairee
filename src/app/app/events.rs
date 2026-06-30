use crate::app::actions::handle_action;
use crate::app::context::AppContext;
use crate::app::input::handle_cli_input;
use crate::app::input_popup::handle_popup_input;
use crate::app::screen_input::handle_screen_input;
use crate::app::state::{AppState, PopupType};
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
            state.current_modifiers = key.modifiers;

            log::debug!("KeyEvent received: {:?}", key);

            // Filter out KeyRelease events on Windows to prevent double-step triggers
            if key.kind == crossterm::event::KeyEventKind::Release {
                return Ok(());
            }

            // Popups consume inputs first
            let popup_active = state.active_popup.is_some();
            match handle_popup_input(state, key, context) {
                Ok(Some(action)) => {
                    handle_action(state, action, context, terminal_backend).await?;
                    return Ok(());
                }
                Ok(None) => {
                    return Ok(());
                }
                Err(()) => {
                    if popup_active {
                        return Ok(());
                    }
                }
            }

            // Screens consume inputs before CLI and Panels (unless it's a global shortcut)
            if handle_screen_input(state, key, context).is_ok() {
                return Ok(());
            }

            if context.config.settings.enable_yazi_workflow && state.cli_input.is_empty() {
                if let crossterm::event::KeyCode::Char(c) = key.code {
                    if key.modifiers.is_empty() {
                        if c == 's' {
                            state.active_popup = Some(PopupType::YaziSortPopup);
                            return Ok(());
                        } else if c == 'v' {
                            state.active_popup = Some(PopupType::YaziViewPopup);
                            return Ok(());
                        }
                    }
                }
            }

            // CLI input takes priority next if applicable
            if handle_cli_input(state, key, context, terminal_backend).is_ok() {
                return Ok(());
            }

            // Standard resolved actions
            let key_str = crate::keybindings::resolver::key_event_to_string(key);
            if !key_str.is_empty() {
                let payload = serde_json::json!({ "key": key_str });
                let _ = tokio::spawn(async move {
                    crate::plugin::hooks::emit_event("on_key", payload).await;
                });
            }

            if let Some(action) = context.resolver.resolve(key) {
                handle_action(state, action, context, terminal_backend).await?;
            } else if !key_str.is_empty() {
                if let Some((plugin_name, action_name)) =
                    crate::plugin::registry::resolve_keybinding(&key_str).await
                {
                    crate::plugin::registry::run_command(&plugin_name, vec![action_name]).await;
                }
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
    Ok(())
}
