use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::fs::attrs::AttrChange;
use crate::fs::vfs::PanelSource;
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};
use std::path::PathBuf;

/// Longest octal mode accepted (e.g. `0755`).
const MODE_DIGITS: usize = 4;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::FileAttributesDialog { attrs, mode_input }) = state.dialogs.top_mut()
    else {
        return Err(());
    };
    match key.code {
        KeyCode::Esc => state.dialogs.clear(),
        KeyCode::Char(c) if c.is_digit(8) => {
            if mode_input.len() < MODE_DIGITS {
                mode_input.push(c);
            }
        }
        KeyCode::Backspace => {
            mode_input.pop();
        }
        KeyCode::Char('r' | 'R' | ' ') => attrs.readonly = !attrs.readonly,
        KeyCode::Enter => {
            let change = AttrChange {
                mode: u32::from_str_radix(mode_input, 8).ok(),
                readonly: attrs.readonly,
            };
            let path = attrs.path.clone();
            let source = attrs.source.clone();
            state.dialogs.clear();
            apply(state, context, source, path, change);
        }
        _ => {}
    }
    Ok(None)
}

/// Sets the octal mode (when typed) and the read-only flag through the
/// entry's source: at once on the local disk, in the background on SFTP.
fn apply(
    state: &mut AppState,
    context: &AppContext,
    source: PanelSource,
    path: PathBuf,
    change: AttrChange,
) {
    let vfs = source.vfs();
    if !source.is_local() {
        state.start_vfs_op(move || vfs.set_attributes(&path, change));
        return;
    }
    match vfs.set_attributes(&path, change) {
        Ok(()) => state.refresh_both_panels(context.config.settings.show_hidden),
        Err(e) => state.dialogs.replace(PopupType::Error(e.to_string())),
    }
}
