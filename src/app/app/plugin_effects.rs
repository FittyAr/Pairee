//! What plugins change besides answering their own requests: the keymap
//! (a plugin loaded or unloaded its commands) and the actions they fire
//! with `pairee.emit`.

use crate::app::actions::handle_action;
use crate::app::context::AppContext;
use crate::app::state::AppState;
use crate::keybindings::{KeybindingResolver, plugin_commands};
use crate::terminal::TerminalBackend;

pub async fn apply(
    state: &mut AppState,
    context: &mut AppContext,
    terminal_backend: &mut TerminalBackend,
) -> anyhow::Result<()> {
    if plugin_commands::take_changed() {
        context.resolver = KeybindingResolver::new(&context.config);
        state.mark_ui_dirty();
    }
    for action in std::mem::take(&mut state.plugins.emitted_actions) {
        handle_action(state, action, context, terminal_backend).await?;
        state.mark_ui_dirty();
    }
    Ok(())
}
