use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::keybindings::Action;
use crossterm::event::{KeyCode, KeyEvent};

const MAX_CURSOR_IDX: usize = 10; // 0=presets list, 1=name, 2=host, 3=port, 4=user, 5=pass, 6=key_path, 7=Connect, 8=Save Preset, 9=Delete Preset, 10=Cancel

pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    if let Some(PopupType::SshConnectPrompt {
        panel,
        input_name,
        input_host,
        input_port,
        input_user,
        input_pass,
        input_key_path,
        cursor_idx,
        selected_preset_idx,
    }) = state.active_popup.clone()
    {
        let mut new_name = input_name;
        let mut new_host = input_host;
        let mut new_port = input_port;
        let mut new_user = input_user;
        let mut new_pass = input_pass;
        let mut new_key_path = input_key_path;
        let mut new_idx = cursor_idx;
        let mut new_selected_preset = selected_preset_idx;

        let update_popup = |s: &mut AppState,
                            name: String,
                            host: String,
                            port: String,
                            user: String,
                            pass: String,
                            kp: String,
                            idx: usize,
                            sp_idx: Option<usize>| {
            s.active_popup = Some(PopupType::SshConnectPrompt {
                panel,
                input_name: name,
                input_host: host,
                input_port: port,
                input_user: user,
                input_pass: pass,
                input_key_path: kp,
                cursor_idx: idx,
                selected_preset_idx: sp_idx,
            });
        };

        match key.code {
            KeyCode::Up => {
                if new_idx == 0 {
                    // Navigate presets list
                    let presets = &context.config.settings.ssh_presets;
                    if !presets.is_empty() {
                        let current_sp = new_selected_preset.unwrap_or(0);
                        let next_sp = if current_sp > 0 {
                            current_sp - 1
                        } else {
                            presets.len() - 1
                        };
                        new_selected_preset = Some(next_sp);
                        let p = &presets[next_sp];
                        new_name = p.name.clone();
                        new_host = p.host.clone();
                        new_port = p.port.clone();
                        new_user = p.username.clone();
                        new_pass = p.password.clone().unwrap_or_default();
                        new_key_path = p.key_path.clone().unwrap_or_default();
                    }
                } else if new_idx >= 7 && new_idx <= 10 {
                    // Up from buttons goes to key path field (index 6)
                    new_idx = 6;
                } else {
                    // Normal fields
                    new_idx = if new_idx > 1 { new_idx - 1 } else { 0 };
                }
                update_popup(
                    state,
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
            KeyCode::Down => {
                if new_idx == 0 {
                    // Navigate presets list
                    let presets = &context.config.settings.ssh_presets;
                    if !presets.is_empty() {
                        let current_sp = new_selected_preset.unwrap_or(0);
                        let next_sp = if current_sp + 1 < presets.len() {
                            current_sp + 1
                        } else {
                            0
                        };
                        new_selected_preset = Some(next_sp);
                        let p = &presets[next_sp];
                        new_name = p.name.clone();
                        new_host = p.host.clone();
                        new_port = p.port.clone();
                        new_user = p.username.clone();
                        new_pass = p.password.clone().unwrap_or_default();
                        new_key_path = p.key_path.clone().unwrap_or_default();
                    }
                } else if new_idx == 6 {
                    // Down from key path goes to Connect button (index 7)
                    new_idx = 7;
                } else if new_idx >= 7 && new_idx <= 10 {
                    // Down from buttons loops back to presets list (index 0)
                    new_idx = 0;
                } else {
                    new_idx += 1;
                }
                update_popup(
                    state,
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
            KeyCode::Left => {
                if new_idx >= 7 && new_idx <= 10 {
                    // Horizontal navigation between buttons
                    new_idx = if new_idx > 7 { new_idx - 1 } else { 10 };
                } else if new_idx > 0 && new_idx <= 6 {
                    // Left from fields moves to presets list (index 0)
                    new_idx = 0;
                }
                update_popup(
                    state,
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
            KeyCode::Right => {
                if new_idx >= 7 && new_idx <= 10 {
                    // Horizontal navigation between buttons
                    new_idx = if new_idx < 10 { new_idx + 1 } else { 7 };
                } else if new_idx == 0 {
                    // Right from presets list moves to Name field (index 1)
                    new_idx = 1;
                }
                update_popup(
                    state,
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
            KeyCode::Tab => {
                new_idx = if new_idx < MAX_CURSOR_IDX {
                    new_idx + 1
                } else {
                    0
                };
                update_popup(
                    state,
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
            KeyCode::BackTab => {
                new_idx = if new_idx > 0 {
                    new_idx - 1
                } else {
                    MAX_CURSOR_IDX
                };
                update_popup(
                    state,
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
            KeyCode::Char(c) => {
                match new_idx {
                    1 => new_name.push(c),
                    2 => new_host.push(c),
                    3 => {
                        if c.is_ascii_digit() {
                            new_port.push(c);
                        }
                    }
                    4 => new_user.push(c),
                    5 => new_pass.push(c),
                    6 => new_key_path.push(c),
                    _ => {}
                }
                update_popup(
                    state,
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
            KeyCode::Backspace => {
                match new_idx {
                    1 => {
                        new_name.pop();
                    }
                    2 => {
                        new_host.pop();
                    }
                    3 => {
                        new_port.pop();
                    }
                    4 => {
                        new_user.pop();
                    }
                    5 => {
                        new_pass.pop();
                    }
                    6 => {
                        new_key_path.pop();
                    }
                    _ => {}
                }
                update_popup(
                    state,
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
            KeyCode::Enter => {
                if new_idx == 0 {
                    // Connect immediately to the selected preset!
                    if new_selected_preset.is_none()
                        && !context.config.settings.ssh_presets.is_empty()
                    {
                        new_selected_preset = Some(0);
                    }
                    if let Some(idx) = new_selected_preset {
                        let presets = &context.config.settings.ssh_presets;
                        if idx < presets.len() {
                            let p = &presets[idx];
                            new_host = p.host.clone();
                            new_port = p.port.clone();
                            new_user = p.username.clone();
                            new_pass = p.password.clone().unwrap_or_default();
                            new_key_path = p.key_path.clone().unwrap_or_default();
                        }
                    }
                } else if new_idx == 10 {
                    // Cancel button
                    state.active_popup = None;
                    return Ok(None);
                } else if new_idx == 8 {
                    // Save Preset button
                    if new_name.trim().is_empty() {
                        state.active_popup =
                            Some(PopupType::Error("Preset name cannot be empty".to_string()));
                        return Ok(None);
                    }
                    if new_host.trim().is_empty() {
                        state.active_popup = Some(PopupType::Error(
                            crate::config::localization::t("error_ssh_host_empty"),
                        ));
                        return Ok(None);
                    }
                    let mut presets = context.config.settings.ssh_presets.clone();
                    let new_p = crate::config::settings::SshPreset {
                        name: new_name.trim().to_string(),
                        host: new_host.trim().to_string(),
                        port: new_port.trim().to_string(),
                        username: new_user.trim().to_string(),
                        password: if new_pass.is_empty() {
                            None
                        } else {
                            Some(new_pass.clone())
                        },
                        key_path: if new_key_path.is_empty() {
                            None
                        } else {
                            Some(new_key_path.clone())
                        },
                    };

                    let mut found_idx = None;
                    for (i, p) in presets.iter().enumerate() {
                        if p.name == new_p.name {
                            found_idx = Some(i);
                            break;
                        }
                    }

                    if let Some(idx) = found_idx {
                        presets[idx] = new_p;
                        new_selected_preset = Some(idx);
                    } else {
                        presets.push(new_p);
                        new_selected_preset = Some(presets.len() - 1);
                    }

                    context.config.settings.ssh_presets = presets;
                    let _ = context.config.save();

                    // Stay on Save button but refresh state
                    update_popup(
                        state,
                        new_name,
                        new_host,
                        new_port,
                        new_user,
                        new_pass,
                        new_key_path,
                        8,
                        new_selected_preset,
                    );
                    return Ok(None);
                } else if new_idx == 9 {
                    // Delete Preset button
                    if let Some(idx) = new_selected_preset {
                        let mut presets = context.config.settings.ssh_presets.clone();
                        if idx < presets.len() {
                            presets.remove(idx);
                            context.config.settings.ssh_presets = presets;
                            let _ = context.config.save();

                            let next_presets = &context.config.settings.ssh_presets;
                            if !next_presets.is_empty() {
                                let next_idx = idx.min(next_presets.len() - 1);
                                let p = &next_presets[next_idx];
                                new_name = p.name.clone();
                                new_host = p.host.clone();
                                new_port = p.port.clone();
                                new_user = p.username.clone();
                                new_pass = p.password.clone().unwrap_or_default();
                                new_key_path = p.key_path.clone().unwrap_or_default();
                                new_selected_preset = Some(next_idx);
                                new_idx = 0; // Focus presets list
                            } else {
                                new_name = String::new();
                                new_host = String::new();
                                new_port = "22".to_string();
                                new_user = String::new();
                                new_pass = String::new();
                                new_key_path = String::new();
                                new_selected_preset = None;
                                new_idx = 1; // Focus Name field
                            }
                        }
                    }
                    update_popup(
                        state,
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

                // If not Cancel/Save/Delete, Enter triggers connect logic
                if new_host.trim().is_empty() {
                    state.active_popup = Some(PopupType::Error(crate::config::localization::t(
                        "error_ssh_host_empty",
                    )));
                    return Ok(None);
                }
                if new_user.trim().is_empty() {
                    state.active_popup = Some(PopupType::Error(crate::config::localization::t(
                        "error_ssh_user_empty",
                    )));
                    return Ok(None);
                }

                let host = new_host.trim().to_string();
                let port_val = new_port.trim().parse::<u16>().unwrap_or(22);
                let user = new_user.trim().to_string();
                let pass = if new_pass.is_empty() {
                    None
                } else {
                    Some(new_pass.clone())
                };
                let key_path = if new_key_path.is_empty() {
                    None
                } else {
                    Some(new_key_path.clone())
                };

                // Create channel
                let (tx, rx) = tokio::sync::oneshot::channel();
                state.ssh_connect_rx = Some(rx);
                state.active_popup = Some(PopupType::Info(crate::config::localization::t(
                    "progress_connecting_ssh",
                )));

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
