use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::update::{UpdateInfo, UpdateStatus};

use tokio::sync::oneshot::error::TryRecvError;

pub fn process_update_events(state: &mut AppState, context: &mut AppContext) {
    // 1.8 Process background update check result
    poll_update_check(state, context);
    // 1.9 Process download progress for ongoing self-update
    poll_download_progress(state);
    // 1.10 Process installation result for self-update
    poll_install_result(state);
}

/// `true` while the "checking for updates" notice is on top.
fn checking_notice_on_top(state: &AppState) -> bool {
    matches!(state.dialogs.top(), Some(PopupType::Info(msg)) if msg == &t("update_checking"))
}

fn poll_update_check(state: &mut AppState, context: &AppContext) {
    let check = state
        .update
        .check_rx
        .as_mut()
        .map(|rx| rx.try_recv())
        .unwrap_or(Err(TryRecvError::Empty));
    let notice = match check {
        Err(TryRecvError::Empty) => return,
        Ok(Some(info)) => {
            state.update.check_rx = None;
            if checking_notice_on_top(state) {
                state.dialogs.clear();
            }
            show_update_found(state, context, info);
            return;
        }
        Ok(None) => "update_no_updates",
        Err(TryRecvError::Closed) => "update_check_failed",
    };
    state.update.check_rx = None;
    if checking_notice_on_top(state) {
        state.dialogs.replace(PopupType::Info(t(notice)));
    }
}

/// Offers the update unless the user dismissed that version.
fn show_update_found(state: &mut AppState, context: &AppContext, info: UpdateInfo) {
    let dismissed = context
        .config
        .settings
        .dismissed_update_version
        .as_deref()
        .is_some_and(|d| d == info.tag);
    if !dismissed {
        state.update.available = Some(info.clone());
        if state.dialogs.is_none() {
            state.dialogs.replace(PopupType::UpdateAvailable {
                info,
                cursor_idx: 0,
                install_progress: None,
                error: None,
                scroll_y: 0,
            });
        }
    } else if state.dialogs.is_none() {
        state.dialogs.replace(PopupType::Info(
            t("update_available_ignored").replace("{}", &info.tag),
        ));
    }
}

fn poll_download_progress(state: &mut AppState) {
    let Some(rx) = state.update.progress_rx.as_mut() else {
        return;
    };
    let mut latest_progress = None;
    let mut disconnected = false;
    loop {
        match rx.try_recv() {
            Ok(p) => latest_progress = Some(p),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                disconnected = true;
                break;
            }
        }
    }
    if let Some(p) = latest_progress {
        if let Some(PopupType::UpdateAvailable {
            install_progress, ..
        }) = state.dialogs.top_mut()
        {
            *install_progress = Some(p);
        }
        state.update.status = UpdateStatus::Downloading(p);
    }
    if disconnected {
        state.update.progress_rx = None;
    }
}

fn poll_install_result(state: &mut AppState) {
    let install = state
        .update
        .install_rx
        .as_mut()
        .map(|rx| rx.try_recv())
        .unwrap_or(Err(TryRecvError::Empty));
    match install {
        Ok(result) => {
            state.update.install_rx = None;
            state.update.progress_rx = None;
            apply_install_result(state, result);
        }
        Err(TryRecvError::Empty) => {
            if state.update.install_rx.is_some()
                && state.update.progress_rx.is_none()
                && state.update.status != UpdateStatus::Installing
            {
                state.update.status = UpdateStatus::Installing;
            }
        }
        Err(TryRecvError::Closed) => {
            state.update.install_rx = None;
            state.update.progress_rx = None;
            state.update.status = UpdateStatus::Error(t("update_installation_task_terminated"));
            show_install_error(state, t("update_installation_task_terminated"));
        }
    }
}

fn apply_install_result(
    state: &mut AppState,
    result: Result<crate::update::installer::InstallResult, String>,
) {
    use crate::update::installer::InstallResult;
    match result {
        Ok(InstallResult::RestartRequired) => {
            state.update.status = UpdateStatus::Done;
            state
                .dialogs
                .replace(PopupType::Info(t("update_installed_restart")));
        }
        Ok(InstallResult::ManagedCommandShown) => {
            state.update.status = UpdateStatus::Done;
        }
        #[cfg(target_os = "windows")]
        Ok(InstallResult::WindowsInstallerLaunched) => {
            state.update.status = UpdateStatus::Done;
            state.should_quit = true;
        }
        Err(err) => {
            state.update.status = UpdateStatus::Error(err.clone());
            if !show_install_error(state, err.clone()) {
                state
                    .dialogs
                    .replace(PopupType::Info(t("update_failed").replace("{}", &err)));
            }
        }
    }
}

/// Puts `err` in the open update dialog; `false` when it is not on top.
fn show_install_error(state: &mut AppState, err: String) -> bool {
    let Some(PopupType::UpdateAvailable {
        error,
        install_progress,
        ..
    }) = state.dialogs.top_mut()
    else {
        return false;
    };
    *error = Some(err);
    *install_progress = None;
    true
}
