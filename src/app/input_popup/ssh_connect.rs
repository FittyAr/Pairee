use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const MAX_CURSOR_IDX: usize = 6; // 0=host, 1=port, 2=user, 3=pass, 4=key_path, 5=OK, 6=Cancel

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    _context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::SshConnectPrompt {
        panel,
        input_host,
        input_port,
        input_user,
        input_pass,
        input_key_path,
        cursor_idx,
    }) = state.active_popup.clone()
    {
        let mut new_host = input_host;
        let mut new_port = input_port;
        let mut new_user = input_user;
        let mut new_pass = input_pass;
        let mut new_key_path = input_key_path;
        let mut new_idx = cursor_idx;

        let update_popup = |s: &mut AppState, host: String, port: String, user: String, pass: String, kp: String, idx: usize| {
            s.active_popup = Some(PopupType::SshConnectPrompt {
                panel,
                input_host: host,
                input_port: port,
                input_user: user,
                input_pass: pass,
                input_key_path: kp,
                cursor_idx: idx,
            });
        };

        match key.code {
            KeyCode::Up | KeyCode::BackTab => {
                new_idx = if new_idx > 0 {
                    new_idx - 1
                } else {
                    MAX_CURSOR_IDX
                };
                update_popup(state, new_host, new_port, new_user, new_pass, new_key_path, new_idx);
                return Ok(None);
            }
            KeyCode::Down | KeyCode::Tab => {
                new_idx = if new_idx < MAX_CURSOR_IDX {
                    new_idx + 1
                } else {
                    0
                };
                update_popup(state, new_host, new_port, new_user, new_pass, new_key_path, new_idx);
                return Ok(None);
            }
            KeyCode::Char(c) => {
                match new_idx {
                    0 => new_host.push(c),
                    1 => {
                        if c.is_ascii_digit() {
                            new_port.push(c);
                        }
                    }
                    2 => new_user.push(c),
                    3 => new_pass.push(c),
                    4 => new_key_path.push(c),
                    _ => {}
                }
                update_popup(state, new_host, new_port, new_user, new_pass, new_key_path, new_idx);
                return Ok(None);
            }
            KeyCode::Backspace => {
                match new_idx {
                    0 => { new_host.pop(); }
                    1 => { new_port.pop(); }
                    2 => { new_user.pop(); }
                    3 => { new_pass.pop(); }
                    4 => { new_key_path.pop(); }
                    _ => {}
                }
                update_popup(state, new_host, new_port, new_user, new_pass, new_key_path, new_idx);
                return Ok(None);
            }
            KeyCode::Enter => {
                if new_idx == 6 {
                    // Cancel
                    state.active_popup = None;
                    return Ok(None);
                }

                // Connect logic (triggered when pressing Enter on OK button, or from any field if not on Cancel)
                if new_host.trim().is_empty() {
                    state.active_popup = Some(PopupType::Error(crate::config::localization::t("error_ssh_host_empty")));
                    return Ok(None);
                }
                if new_user.trim().is_empty() {
                    state.active_popup = Some(PopupType::Error(crate::config::localization::t("error_ssh_user_empty")));
                    return Ok(None);
                }
 
                let host = new_host.trim().to_string();
                let port_val = new_port.trim().parse::<u16>().unwrap_or(22);
                let user = new_user.trim().to_string();
                let pass = if new_pass.is_empty() { None } else { Some(new_pass.clone()) };
                let key_path = if new_key_path.is_empty() { None } else { Some(new_key_path.clone()) };
 
                // Create channel
                let (tx, rx) = tokio::sync::oneshot::channel();
                state.ssh_connect_rx = Some(rx);
                state.active_popup = Some(PopupType::Info(crate::config::localization::t("progress_connecting_ssh")));

                tokio::spawn(async move {
                    let res = crate::fs::ssh::SharedSshClient::connect(
                        &host,
                        port_val,
                        &user,
                        pass.as_deref(),
                        key_path.as_deref(),
                    );
                    let _ = tx.send((panel, res));
                });

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
