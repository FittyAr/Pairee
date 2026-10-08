//! Enter in the SSH connection dialog: pick a preset, save / delete a
//! preset, cancel or connect.

use crate::app::context::AppContext;
use crate::app::state::popup::SshField;
use crate::app::state::{AppState, PopupType, SshConnectPromptState as Prompt};
use crate::config::localization::t;

pub fn handle_enter(state: &mut AppState, context: &mut AppContext) {
    let Some(PopupType::SshConnectPrompt(prompt)) = state.dialogs.top_mut() else {
        return;
    };
    match prompt.cursor_idx {
        Prompt::ROW_PRESETS => {
            let presets = &context.config.settings.ssh_presets;
            let idx = prompt.selected_preset_idx.unwrap_or(0);
            match presets.get(idx) {
                Some(preset) => prompt.load_preset(idx, preset),
                None => connect(state, context),
            }
        }
        Prompt::BUTTON_CANCEL => state.dialogs.clear(),
        Prompt::BUTTON_SAVE => {
            if let Err(key) = save_preset(prompt, context) {
                state.dialogs.replace(PopupType::Error(t(key)));
            }
        }
        Prompt::BUTTON_DELETE => delete_preset(prompt, context),
        _ => connect(state, context),
    }
}

/// Saves the fields as a preset (replacing one with the same name).
/// `Err` holds the message key of a missing required field.
fn save_preset(prompt: &mut Prompt, context: &mut AppContext) -> Result<(), &'static str> {
    let preset = prompt.to_preset();
    if preset.name.is_empty() {
        return Err("error_ssh_preset_name_empty");
    }
    if preset.host.is_empty() {
        return Err("error_ssh_host_empty");
    }
    let presets = &mut context.config.settings.ssh_presets;
    let idx = match presets.iter().position(|p| p.name == preset.name) {
        Some(idx) => {
            presets[idx] = preset;
            idx
        }
        None => {
            presets.push(preset);
            presets.len() - 1
        }
    };
    prompt.selected_preset_idx = Some(idx);
    context.config.save_logging();
    Ok(())
}

/// Deletes the selected preset and shows the next one (or empty fields).
fn delete_preset(prompt: &mut Prompt, context: &mut AppContext) {
    let Some(idx) = prompt.selected_preset_idx else {
        return;
    };
    let presets = &mut context.config.settings.ssh_presets;
    if idx >= presets.len() {
        return;
    }
    presets.remove(idx);
    match presets.len() {
        0 => {
            prompt.clear_fields();
            prompt.cursor_idx = SshField::Name.row();
        }
        len => {
            let next = idx.min(len - 1);
            prompt.load_preset(next, &presets[next]);
            prompt.cursor_idx = Prompt::ROW_PRESETS;
        }
    }
    context.config.save_logging();
}

/// Validates the fields and connects in the background.
fn connect(state: &mut AppState, context: &AppContext) {
    let Some(PopupType::SshConnectPrompt(prompt)) = state.dialogs.top() else {
        return;
    };
    let preset = prompt.to_preset();
    let panel = state.panels.active_tab_id(prompt.panel);
    if preset.host.is_empty() {
        state
            .dialogs
            .replace(PopupType::Error(t("error_ssh_host_empty")));
        return;
    }
    if preset.username.is_empty() {
        state
            .dialogs
            .replace(PopupType::Error(t("error_ssh_user_empty")));
        return;
    }
    let port = preset.port.parse::<u16>().unwrap_or(22);
    let timeout = std::time::Duration::from_secs(context.config.settings.ssh_timeout_secs);
    state
        .dialogs
        .replace(PopupType::Info(t("progress_connecting_ssh")));

    // Blocking handshake/auth: run on the job pool, applied by
    // `process_ssh_connect_updates` on the UI thread.
    state.ssh_connect.start(move |_| {
        let res = crate::fs::ssh::SharedSshClient::connect(
            &preset.host,
            port,
            &preset.username,
            preset.password.as_deref(),
            preset.key_path.as_deref(),
            timeout,
        );
        (panel, res)
    });
}
