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
    }) = state.active_popup.clone()
    {
        match key.code {
            KeyCode::Char(c) => {
                let mut new_input = input;
                new_input.push(c);
                state.active_popup = Some(PopupType::DescribeFilePrompt {
                    path,
                    current_desc,
                    input: new_input,
                });
                return Ok(None);
            }
            KeyCode::Backspace => {
                let mut new_input = input;
                new_input.pop();
                state.active_popup = Some(PopupType::DescribeFilePrompt {
                    path,
                    current_desc,
                    input: new_input,
                });
                return Ok(None);
            }
            KeyCode::Enter => {
                state.active_popup = None;
                if let Some(dir) = path.parent()
                    && let Some(name) = path.file_name()
                {
                    let _ = crate::fs::write_description(dir, &name.to_string_lossy(), &input);
                }
                state.refresh_both_panels(context.config.settings.show_hidden);
                return Ok(None);
            }
            KeyCode::Esc => {
                state.active_popup = None;
                return Ok(None);
            }
            _ => {}
        }
        Err(())
    } else {
        Err(())
    }
}
