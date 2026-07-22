use crate::app::context::AppContext;
use crate::app::state::{AppState, PopupType};
use crate::config::localization::t;
use crate::update::UpdateStatus;

pub fn process_update_events(state: &mut AppState, context: &mut AppContext) {
    // 1.8 Process background update check result
    if state.update_check_rx.is_some() {
        let mut rx = state.update_check_rx.take().unwrap();
        match rx.try_recv() {
            Ok(Some(info)) => {
                // If we had a "Checking for updates..." popup active, dismiss it
                if let Some(PopupType::Info(ref msg)) = state.active_popup {
                    if msg == &t("update_checking") {
                        state.active_popup = None;
                    }
                }

                // Don't show if the user already dismissed this version
                let dismissed = context
                    .config
                    .settings
                    .dismissed_update_version
                    .as_deref()
                    .map(|d| d == info.tag)
                    .unwrap_or(false);
                if !dismissed {
                    state.update_available = Some(info.clone());
                    // Only show popup if no other popup is active
                    if state.active_popup.is_none() {
                        state.active_popup = Some(PopupType::UpdateAvailable {
                            info,
                            cursor_idx: 0,
                            install_progress: None,
                            error: None,
                            scroll_y: 0,
                        });
                    }
                } else {
                    // If it's already dismissed and we forced a check, we still show a message
                    if state.active_popup.is_none() {
                        state.active_popup = Some(PopupType::Info(
                            t("update_available_ignored").replace("{}", &info.tag),
                        ));
                    }
                }
            }
            Ok(None) => {
                // No update available. If we had "Checking for updates..." popup, show info.
                if let Some(PopupType::Info(ref msg)) = state.active_popup {
                    if msg == &t("update_checking") {
                        state.active_popup = Some(PopupType::Info(t("update_no_updates")));
                    }
                }
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                state.update_check_rx = Some(rx); // Still waiting
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                // Channel closed / check failed. If we had "Checking for updates..." popup, show error.
                if let Some(PopupType::Info(ref msg)) = state.active_popup {
                    if msg == &t("update_checking") {
                        state.active_popup = Some(PopupType::Info(t("update_check_failed")));
                    }
                }
            }
        }
    }

    // 1.9 Process download progress for ongoing self-update
    if state.update_progress_rx.is_some() {
        let mut rx = state.update_progress_rx.take().unwrap();
        let mut latest_progress = None;
        loop {
            match rx.try_recv() {
                Ok(p) => {
                    latest_progress = Some(p);
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
            }
        }
        if let Some(p) = latest_progress {
            if let Some(PopupType::UpdateAvailable {
                install_progress, ..
            }) = &mut state.active_popup
            {
                *install_progress = Some(p);
            }
            state.update_status = UpdateStatus::Downloading(p);
        }
        state.update_progress_rx = Some(rx);
    }

    // 1.10 Process installation result for self-update
    if state.update_install_rx.is_some() {
        let mut rx = state.update_install_rx.take().unwrap();
        match rx.try_recv() {
            Ok(result) => {
                // Task finished
                state.update_progress_rx = None; // Clean up progress rx
                match result {
                    Ok(crate::update::installer::InstallResult::RestartRequired) => {
                        state.update_status = UpdateStatus::Done;
                        state.active_popup = Some(PopupType::Info(t("update_installed_restart")));
                    }
                    Ok(crate::update::installer::InstallResult::ManagedCommandShown) => {
                        state.update_status = UpdateStatus::Done;
                    }
                    #[cfg(target_os = "windows")]
                    Ok(crate::update::installer::InstallResult::WindowsInstallerLaunched) => {
                        state.update_status = UpdateStatus::Done;
                        state.should_quit = true; // Quit so installer can run
                    }
                    Err(err) => {
                        state.update_status = UpdateStatus::Error(err.clone());
                        if let Some(PopupType::UpdateAvailable {
                            error,
                            install_progress,
                            ..
                        }) = &mut state.active_popup
                        {
                            *error = Some(err);
                            *install_progress = None;
                        } else {
                            state.active_popup =
                                Some(PopupType::Info(t("update_failed").replace("{}", &err)));
                        }
                    }
                }
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                state.update_install_rx = Some(rx); // Still running
                // If download is complete (or not active), set status to Installing
                if state.update_progress_rx.is_none()
                    && state.update_status != UpdateStatus::Installing
                {
                    state.update_status = UpdateStatus::Installing;
                }
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                // Task died/panicked
                state.update_progress_rx = None;
                state.update_status = UpdateStatus::Error(t("update_installation_task_terminated"));
                if let Some(PopupType::UpdateAvailable {
                    error,
                    install_progress,
                    ..
                }) = &mut state.active_popup
                {
                    *error = Some(t("update_installation_task_terminated"));
                    *install_progress = None;
                }
            }
        }
    }
}
