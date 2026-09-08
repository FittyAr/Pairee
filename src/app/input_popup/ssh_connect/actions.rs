//! Button and Enter action handlers for the SSH connection prompt.

use crate::app::context::AppContext;
use crate::app::state::{ActivePanel, AppState, PopupType};
use crate::config::localization::t;

pub fn handle_enter(
    state: &mut AppState,
    context: &mut AppContext,
    panel: ActivePanel,
    mut new_name: String,
    mut new_host: String,
    new_port: String,
    new_user: String,
    new_pass: String,
    new_key_path: String,
    mut new_idx: usize,
    mut new_selected_preset: Option<usize>,
) {
    if new_idx == 0 {
        if new_selected_preset.is_none() && !context.config.settings.ssh_presets.is_empty() {
            new_selected_preset = Some(0);
        }
        if let Some(idx) = new_selected_preset {
            let presets = &context.config.settings.ssh_presets;
            if idx < presets.len() {
                let p = &presets[idx];
                new_host = p.host.clone();
                let new_port_clone = p.port.clone();
                let new_user_clone = p.username.clone();
                let new_pass_clone = p.password.clone().unwrap_or_default();
                let new_key_path_clone = p.key_path.clone().unwrap_or_default();
                state.dialogs.replace(PopupType::SshConnectPrompt(
                    crate::app::state::SshConnectPromptState {
                        panel,
                        input_name: new_name,
                        input_host: new_host,
                        input_port: new_port_clone,
                        input_user: new_user_clone,
                        input_pass: new_pass_clone,
                        input_key_path: new_key_path_clone,
                        cursor_idx: new_idx,
                        selected_preset_idx: new_selected_preset,
                    },
                ));
                return;
            }
        }
    } else if new_idx == 10 {
        state.dialogs.clear();
        return;
    } else if new_idx == 8 {
        if new_name.trim().is_empty() {
            state
                .dialogs
                .replace(PopupType::Error(t("error_ssh_preset_name_empty")));
            return;
        }
        if new_host.trim().is_empty() {
            state
                .dialogs
                .replace(PopupType::Error(t("error_ssh_host_empty")));
            return;
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
        context.config.save_logging();

        state.dialogs.replace(PopupType::SshConnectPrompt(
            crate::app::state::SshConnectPromptState {
                panel,
                input_name: new_name,
                input_host: new_host,
                input_port: new_port,
                input_user: new_user,
                input_pass: new_pass,
                input_key_path: new_key_path,
                cursor_idx: 8,
                selected_preset_idx: new_selected_preset,
            },
        ));
        return;
    } else if new_idx == 9 {
        let Some(idx) = new_selected_preset else {
            return;
        };
        let mut presets = context.config.settings.ssh_presets.clone();
        if idx < presets.len() {
            presets.remove(idx);
            context.config.settings.ssh_presets = presets;
            context.config.save_logging();

            let next_presets = &context.config.settings.ssh_presets;
            if !next_presets.is_empty() {
                let next_idx = idx.min(next_presets.len() - 1);
                let p = &next_presets[next_idx];
                new_name = p.name.clone();
                new_host = p.host.clone();
                let new_port_val = p.port.clone();
                let new_user_val = p.username.clone();
                let new_pass_val = p.password.clone().unwrap_or_default();
                let new_key_path_val = p.key_path.clone().unwrap_or_default();
                new_selected_preset = Some(next_idx);
                new_idx = 0;
                state.dialogs.replace(PopupType::SshConnectPrompt(
                    crate::app::state::SshConnectPromptState {
                        panel,
                        input_name: new_name,
                        input_host: new_host,
                        input_port: new_port_val,
                        input_user: new_user_val,
                        input_pass: new_pass_val,
                        input_key_path: new_key_path_val,
                        cursor_idx: new_idx,
                        selected_preset_idx: new_selected_preset,
                    },
                ));
                return;
            } else {
                new_name = String::new();
                new_host = String::new();
                new_selected_preset = None;
                new_idx = 1;
                state.dialogs.replace(PopupType::SshConnectPrompt(
                    crate::app::state::SshConnectPromptState {
                        panel,
                        input_name: new_name,
                        input_host: new_host,
                        input_port: "22".to_string(),
                        input_user: String::new(),
                        input_pass: String::new(),
                        input_key_path: String::new(),
                        cursor_idx: new_idx,
                        selected_preset_idx: new_selected_preset,
                    },
                ));
                return;
            }
        }
    }

    if new_host.trim().is_empty() {
        state
            .dialogs
            .replace(PopupType::Error(t("error_ssh_host_empty")));
        return;
    }
    if new_user.trim().is_empty() {
        state
            .dialogs
            .replace(PopupType::Error(t("error_ssh_user_empty")));
        return;
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

    let (tx, rx) = tokio::sync::oneshot::channel();
    state.ssh_connect_rx = Some(rx);
    state
        .dialogs
        .replace(PopupType::Info(t("progress_connecting_ssh")));

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
}
