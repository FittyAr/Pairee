use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::update::UpdateStatus;

pub fn process_update_events(state: &mut AppState, context: &mut AppContext) {
    // 1.8 Process background update check result
    let check = state
        .update
        .check_rx
        .as_mut()
        .map(|rx| rx.try_recv())
        .unwrap_or(Err(tokio::sync::oneshot::error::TryRecvError::Empty));
    match check {
        Ok(Some(info)) => {
            state.update.check_rx = None;
            if let Some(PopupType::Info(msg)) = state.dialogs.top()
                && msg == &t("update_checking")
            {
                state.dialogs.clear();
            }

            let dismissed = context
                .config
                .settings
                .dismissed_update_version
                .as_deref()
                .map(|d| d == info.tag)
                .unwrap_or(false);
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
        Ok(None) => {
            state.update.check_rx = None;
            if let Some(PopupType::Info(msg)) = state.dialogs.top()
                && msg == &t("update_checking")
            {
                state
                    .dialogs
                    .replace(PopupType::Info(t("update_no_updates")));
            }
        }
        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {}
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
            state.update.check_rx = None;
            if let Some(PopupType::Info(msg)) = state.dialogs.top()
                && msg == &t("update_checking")
            {
                state
                    .dialogs
                    .replace(PopupType::Info(t("update_check_failed")));
            }
        }
    }

    // 1.9 Process download progress for ongoing self-update
    if let Some(rx) = state.update.progress_rx.as_mut() {
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

    // 1.10 Process installation result for self-update
    let install = state
        .update
        .install_rx
        .as_mut()
        .map(|rx| rx.try_recv())
        .unwrap_or(Err(tokio::sync::oneshot::error::TryRecvError::Empty));
    match install {
        Ok(result) => {
            state.update.install_rx = None;
            state.update.progress_rx = None;
            match result {
                Ok(crate::update::installer::InstallResult::RestartRequired) => {
                    state.update.status = UpdateStatus::Done;
                    state
                        .dialogs
                        .replace(PopupType::Info(t("update_installed_restart")));
                }
                Ok(crate::update::installer::InstallResult::ManagedCommandShown) => {
                    state.update.status = UpdateStatus::Done;
                }
                #[cfg(target_os = "windows")]
                Ok(crate::update::installer::InstallResult::WindowsInstallerLaunched) => {
                    state.update.status = UpdateStatus::Done;
                    state.should_quit = true;
                }
                Err(err) => {
                    state.update.status = UpdateStatus::Error(err.clone());
                    if let Some(PopupType::UpdateAvailable {
                        error,
                        install_progress,
                        ..
                    }) = state.dialogs.top_mut()
                    {
                        *error = Some(err);
                        *install_progress = None;
                    } else {
                        state
                            .dialogs
                            .replace(PopupType::Info(t("update_failed").replace("{}", &err)));
                    }
                }
            }
        }
        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
            if state.update.install_rx.is_some()
                && state.update.progress_rx.is_none()
                && state.update.status != UpdateStatus::Installing
            {
                state.update.status = UpdateStatus::Installing;
            }
        }
        Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
            state.update.install_rx = None;
            state.update.progress_rx = None;
            state.update.status = UpdateStatus::Error(t("update_installation_task_terminated"));
            if let Some(PopupType::UpdateAvailable {
                error,
                install_progress,
                ..
            }) = state.dialogs.top_mut()
            {
                *error = Some(t("update_installation_task_terminated"));
                *install_progress = None;
            }
        }
    }
}
