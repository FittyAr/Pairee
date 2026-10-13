use crate::app::context::AppContext;
use crate::app::form::{FieldKey, field_key};
use crate::app::state::AppState;
use crate::app::state::popup::DescribeFilePrompt;
use crate::app::state::popup::{Dialog, KeyHandler};
use crate::app::text_input::TextField;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let Some(DescribeFilePrompt { input, .. }) = state.dialogs.top_mut_as::<DescribeFilePrompt>()
    else {
        return Err(());
    };
    match field_key(input, &key) {
        FieldKey::Cancel => state.dialogs.clear(),
        FieldKey::Submit => {
            if let Some(DescribeFilePrompt { path, input, .. }) =
                state.dialogs.pop_as::<DescribeFilePrompt>()
            {
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

impl Dialog for DescribeFilePrompt {
    fn key_handler(&self) -> KeyHandler {
        handle
    }
    fn focused_field_mut(&mut self) -> Option<&mut TextField> {
        Some(&mut self.input)
    }
}
