use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::keybindings::Action;
use crate::update::{UpdateStatus, detect::detect_install_method, installer};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

/// Handle keyboard input for the UpdateAvailable popup.
/// Returns Ok(None) if consumed, Ok(Some(action)) to bubble up, Err(()) to ignore.
pub fn handle(
    state: &mut AppState,
    key: KeyEvent,
    context: &mut AppContext,
) -> Result<Option<Action>, ()> {
    let (info_clone, cursor_idx) = match state.dialogs.top() {
        Some(PopupType::UpdateAvailable {
            info,
            cursor_idx,
            install_progress,
            ..
        }) => {
            // Block navigation while installing, but allow Esc or 'q' to close the popup
            if install_progress.is_some() {
                if key.code == KeyCode::Esc || key.code == KeyCode::Char('q') {
                    state.dialogs.clear();
                }
                return Ok(None);
            }
            (info.clone(), *cursor_idx)
        }
        _ => return Err(()),
    };

    match key.code {
        KeyCode::Enter => activate_button(state, context, info_clone, cursor_idx),
        KeyCode::Esc | KeyCode::Char('q') => state.dialogs.clear(),
        code => move_in_popup(state, code),
    }
    Ok(None)
}

/// Scrolls the release notes (`Up`/`Down`/`PgUp`/`PgDn`, `j`/`k`) or moves
/// between the buttons (`Left`/`Right`/`Tab`, `h`/`l`).
fn move_in_popup(state: &mut AppState, code: KeyCode) {
    let Some(PopupType::UpdateAvailable {
        scroll_y,
        cursor_idx,
        ..
    }) = state.dialogs.top_mut()
    else {
        return;
    };
    match code {
        KeyCode::Up | KeyCode::Char('k' | 'K') => *scroll_y = scroll_y.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j' | 'J') => *scroll_y = scroll_y.saturating_add(1),
        KeyCode::PageUp => *scroll_y = scroll_y.saturating_sub(5),
        KeyCode::PageDown => *scroll_y = scroll_y.saturating_add(5),
        KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
            *cursor_idx = cursor_idx.saturating_sub(1);
        }
        KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => {
            *cursor_idx = (*cursor_idx + 1).min(2);
        }
        _ => {}
    }
}

fn activate_button(
    state: &mut AppState,
    context: &mut AppContext,
    info: crate::update::UpdateInfo,
    cursor_idx: usize,
) {
    match cursor_idx {
        // 0 = "Update now" / "Copy command"
        0 => {
            let method = detect_install_method();
            if method.is_managed() {
                copy_managed_command(state, &method);
            } else {
                start_self_update(state, info, method);
            }
        }
        // 1 = "Remind me later" — close popup, will show again next session
        1 => state.dialogs.clear(),
        // 2 = "Ignore this version" — save dismissed tag to settings
        2 => {
            context.config.settings.dismissed_update_version = Some(info.tag.clone());
            context.config.save_logging();
            state.dialogs.clear();
            state.update.available = None;
        }
        _ => {}
    }
}

/// Package-manager installs: copies the upgrade command to the clipboard.
fn copy_managed_command(state: &mut AppState, method: &crate::update::detect::InstallMethod) {
    let Some(cmd) = method.managed_upgrade_command() else {
        return;
    };
    match crate::app::sys_helpers::clipboard::set_text(&cmd) {
        Ok(()) => {
            state.dialogs.clear();
            state
                .dialogs
                .replace(PopupType::Info(t("update_cmd_copied").replace("{}", &cmd)));
        }
        Err(e) => {
            state
                .dialogs
                .replace(PopupType::Error(t("clipboard_failed").replace("{}", &e)));
        }
    }
}

/// Downloads and installs the update in the background, reporting progress.
fn start_self_update(
    state: &mut AppState,
    info: crate::update::UpdateInfo,
    method: crate::update::detect::InstallMethod,
) {
    let (progress_tx, progress_rx) = mpsc::channel::<f32>(64);
    state.update.progress_rx = Some(progress_rx);
    state.update.status = UpdateStatus::Downloading(0.0);

    // Set the progress marker in popup
    if let Some(PopupType::UpdateAvailable {
        install_progress, ..
    }) = state.dialogs.top_mut()
    {
        *install_progress = Some(0.0);
    }

    let (install_tx, install_rx) = tokio::sync::oneshot::channel();
    state.update.install_rx = Some(install_rx);

    tokio::spawn(async move {
        let result = installer::perform_update(&info, &method, progress_tx).await;
        let _ = install_tx.send(result.map_err(|e| e.to_string()));
    });
}
