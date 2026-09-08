//! SSH Connection popup input handler orchestrator.

pub mod actions;
pub mod editing;
pub mod navigation;

use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::SshConnectPrompt(crate::app::state::SshConnectPromptState {
        panel,
        input_name,
        input_host,
        input_port,
        input_user,
        input_pass,
        input_key_path,
        cursor_idx,
        selected_preset_idx,
    })) = state.dialogs.top().cloned()
    {
        let mut new_name = input_name;
        let mut new_host = input_host;
        let mut new_port = input_port;
        let mut new_user = input_user;
        let mut new_pass = input_pass;
        let mut new_key_path = input_key_path;
        let mut new_idx = cursor_idx;
        let mut new_selected_preset = selected_preset_idx;

        match key.code {
            KeyCode::Up => {
                navigation::navigate_up(
                    &mut new_idx,
                    &mut new_selected_preset,
                    &context.config.settings.ssh_presets,
                    &mut new_name,
                    &mut new_host,
                    &mut new_port,
                    &mut new_user,
                    &mut new_pass,
                    &mut new_key_path,
                );
            }
            KeyCode::Down => {
                navigation::navigate_down(
                    &mut new_idx,
                    &mut new_selected_preset,
                    &context.config.settings.ssh_presets,
                    &mut new_name,
                    &mut new_host,
                    &mut new_port,
                    &mut new_user,
                    &mut new_pass,
                    &mut new_key_path,
                );
            }
            KeyCode::Left => {
                navigation::navigate_left(&mut new_idx);
            }
            KeyCode::Right => {
                navigation::navigate_right(&mut new_idx);
            }
            KeyCode::Tab => {
                navigation::navigate_tab(&mut new_idx);
            }
            KeyCode::BackTab => {
                navigation::navigate_backtab(&mut new_idx);
            }
            KeyCode::Char(c) => {
                editing::handle_char(
                    c,
                    new_idx,
                    &mut new_name,
                    &mut new_host,
                    &mut new_port,
                    &mut new_user,
                    &mut new_pass,
                    &mut new_key_path,
                );
            }
            KeyCode::Backspace => {
                editing::handle_backspace(
                    new_idx,
                    &mut new_name,
                    &mut new_host,
                    &mut new_port,
                    &mut new_user,
                    &mut new_pass,
                    &mut new_key_path,
                );
            }
            KeyCode::Enter => {
                actions::handle_enter(
                    state,
                    context,
                    panel,
                    new_name,
                    new_host,
                    new_port,
                    new_user,
                    new_pass,
                    new_key_path,
                    new_idx,
                    new_selected_preset,
                );
                return Ok(None);
            }
            KeyCode::Esc => {
                state.dialogs.clear();
                return Ok(None);
            }
            _ => return Err(()),
        }

        state.dialogs.replace(PopupType::SshConnectPrompt(
            crate::app::state::SshConnectPromptState {
                panel,
                input_name: new_name,
                input_host: new_host,
                input_port: new_port,
                input_user: new_user,
                input_pass: new_pass,
                input_key_path: new_key_path,
                cursor_idx: new_idx,
                selected_preset_idx: new_selected_preset,
            },
        ));
        Ok(None)
    } else {
        Err(())
    }
}
