use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(PopupType::DescribeFilePrompt { input, .. }) = state.dialogs.top_mut() else {
        return Err(());
    };
    match field_key(input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(PopupType::DescribeFilePrompt { path, input, .. }) = state.dialogs.pop() {
                state.dialogs.clear();
                if let (Some(dir), Some(name)) = (path.parent(), path.file_name()) {
                    let _ =
                        crate::fs::write_description(dir, &name.to_string_lossy(), input.text());
                }
                state.refresh_both_panels(context.config.settings.show_hidden);
            }
        }
        FieldKey::Handled | FieldKey::Other => {}
    }
    Ok(None)
}
