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
                // §P3 privacy filter: do NOT broadcast `on_key` to
                // any plugin while the user is interacting with a
                // popup. The previous filter only matched
                // `PluginInputDialog { obscure: true, .. }`, but
                // other popups also capture sensitive keys:
                // `y`/`n` in `ConfirmDelete`, `SshConnectPrompt`
                // (SSH password), the `MkDirPrompt` path being
                // typed, etc. The simpler "no popup active" rule
                // is the right semantic: `on_key` means "the user
                // pressed a key in the main loop", not "a popup
                // handler consumed it". The
                // `hooks::emit_event` Secure-Mode filter still
                // suppresses delivery to untrusted plugins in
                // Secure Mode.
                let popup_active = state.active_popup.is_some();
                if !popup_active {
                    let payload = serde_json::json!({ "key": key_str });
                    let _ = tokio::spawn(async move {
                        crate::plugin::hooks::emit_event("on_key", payload).await;
                    });
                }
            }

            // Single dispatch path: every keypress goes through
            // the resolver. The `ResolvedBinding` carries source
            // attribution so we know whether to run the built-in
            // action handler or to forward the keypress to a
            // plugin (manifest entry or Lua runtime bind).
            //
            // This replaces the old split where the preset won
            // outright and the plugin registry was a fallback;
            // that left plugin keybindings such as F2 /
            // Ctrl+R / Ctrl+D silently dead whenever the preset
            // happened to own the key. The new priority table is
            // documented in `crate::keybindings::source`.
            match context.resolver.resolve(key) {
                Some(binding) => match binding.action {
                    crate::keybindings::Action::PluginCommand => {
                        // Manifest entry or Lua runtime bind —
                        // forward to the plugin that registered
                        // the key. The plugin name and action
                        // name travel together on the binding so
                        // there is no string-key indirection.
                        let plugin_name = match &binding.source {
                            crate::keybindings::BindingSource::Plugin { plugin }
                            | crate::keybindings::BindingSource::Lua { plugin } => plugin.clone(),
                            // The sentinel `PluginCommand` is
                            // never produced by Builtin / User
                            // sources today, but if a future
                            // refactor ever does, route it to a
                            // debug log rather than a panic.
                            _ => {
                                log::warn!(
                                    "PluginCommand action with non-plugin source: {:?}",
                                    binding.source
                                );
                                String::new()
                            }
                        };
                        if !plugin_name.is_empty() {
                            crate::plugin::registry::run_command(
                                &plugin_name,
                                vec![binding.plugin_action.clone()],
                            )
                            .await;
                        }
                    }
                    _ => {
                        // Builtin or User binding — the action
                        // itself is the dispatcher key.
                        handle_action(state, binding.action, context, terminal_backend).await?;
                    }
                },
                None => {}
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
