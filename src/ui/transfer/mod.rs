pub mod bar;
pub mod conflict_dialog;
mod labels;
pub mod panel;

use crate::fs::transfer::job::TransferJobStatus;
use crate::ui::resolved_theme::ResolvedTheme;
use ratatui::style::Color;

/// Color of a job status (job list, progress gauge of the inspector).
pub(crate) fn status_color(theme: &ResolvedTheme, status: TransferJobStatus) -> Color {
    let ui = &theme.ui;
    match status {
        TransferJobStatus::Queued => ui.secondary,
        TransferJobStatus::Scanning => ui.accent,
        TransferJobStatus::Transferring => ui.success,
        TransferJobStatus::Verifying => ui.info,
        TransferJobStatus::Paused => ui.warning,
        TransferJobStatus::Completed => ui.success_alt,
        TransferJobStatus::Failed => ui.error,
        TransferJobStatus::Cancelled => ui.danger,
    }
}

#[cfg(test)]
mod tests;
