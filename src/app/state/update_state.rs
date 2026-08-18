use crate::update::{UpdateInfo, UpdateStatus, installer::InstallResult};

/// Self-update check / download / install channels and snapshot.
pub struct UpdateState {
    /// Pending oneshot receiver for the background update check.
    pub check_rx: Option<tokio::sync::oneshot::Receiver<Option<UpdateInfo>>>,
    /// Available update info (set after the background check completes).
    pub available: Option<UpdateInfo>,
    /// Current status of an ongoing update installation.
    pub status: UpdateStatus,
    /// Receiver for download progress (0.0–1.0).
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<f32>>,
    /// Pending oneshot receiver for the final installation result.
    pub install_rx: Option<tokio::sync::oneshot::Receiver<Result<InstallResult, String>>>,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            check_rx: None,
            available: None,
            status: UpdateStatus::Idle,
            progress_rx: None,
            install_rx: None,
        }
    }
}
