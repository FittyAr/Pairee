use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::DescribeFilePrompt {
        path,
        current_desc,
        input,
    }) = state.dialogs.top().cloned()
    {
        match key.code {
            KeyCode::Char(c) => {
                let mut new_input = input;
                new_input.push(c);
                state.dialogs.replace(PopupType::DescribeFilePrompt {
                    path,
                    current_desc,
                    input: new_input,
                });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_input = input;
                new_input.pop();
                state.dialogs.replace(PopupType::DescribeFilePrompt {
                    path,
                    current_desc,
                    input: new_input,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                state.dialogs.clear();
                if let Some(dir) = path.parent()
                    && let Some(name) = path.file_name()
                {
                    let _ = crate::fs::write_description(dir, &name.to_string_lossy(), &input);
                }
                state.refresh_both_panels(context.config.settings.show_hidden);
                return Ok(None);
            }
            KeyCode::Esc => {
                state.dialogs.clear();
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
