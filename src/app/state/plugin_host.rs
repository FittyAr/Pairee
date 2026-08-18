use super::types::DevProgress;
use crate::plugin::manager::InputDialogResult;
use tokio::sync::oneshot;

/// Oneshot waiting for the user to finish a plugin dialog.
///
/// Stored here (not on [`PopupType`]) because senders are not `Clone`.
pub enum PendingPluginReply {
    Confirm(oneshot::Sender<bool>),
    Input(oneshot::Sender<InputDialogResult>),
    /// Legacy `pairee.app.input` — returns the raw string only.
    LegacyInput(oneshot::Sender<String>),
    Which(oneshot::Sender<Option<usize>>),
}

/// Plugin-manager / developer-tools runtime channels.
#[derive(Default)]
pub struct PluginHostState {
    /// Drained each frame to update PluginMenu progress without blocking UI.
    pub dev_progress_rx: Option<tokio::sync::mpsc::UnboundedReceiver<DevProgress>>,
    /// Reply channel for the active plugin confirm/input/which dialog.
    pub pending_dialog: Option<PendingPluginReply>,
}

impl PluginHostState {
    /// Completes a pending dialog as cancelled (Esc / replaced / orphaned).
    pub fn cancel_pending(&mut self) {
        match self.pending_dialog.take() {
            Some(PendingPluginReply::Confirm(tx)) => {
                let _ = tx.send(false);
            }
            Some(PendingPluginReply::Input(tx)) => {
                let _ = tx.send(InputDialogResult {
                    value: String::new(),
                    event: 2,
                });
            }
            Some(PendingPluginReply::LegacyInput(tx)) => {
                let _ = tx.send(String::new());
            }
            Some(PendingPluginReply::Which(tx)) => {
                let _ = tx.send(None);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_pending_confirm_sends_false() {
        let mut host = PluginHostState::default();
        let (tx, rx) = oneshot::channel();
        host.pending_dialog = Some(PendingPluginReply::Confirm(tx));
        host.cancel_pending();
        assert!(host.pending_dialog.is_none());
        assert!(!rx.blocking_recv().unwrap());
    }

    #[test]
    fn cancel_pending_input_sends_cancelled_event() {
        let mut host = PluginHostState::default();
        let (tx, rx) = oneshot::channel();
        host.pending_dialog = Some(PendingPluginReply::Input(tx));
        host.cancel_pending();
        let result = rx.blocking_recv().unwrap();
        assert_eq!(result.value, "");
        assert_eq!(result.event, 2);
    }

    #[test]
    fn cancel_pending_which_sends_none() {
        let mut host = PluginHostState::default();
        let (tx, rx) = oneshot::channel();
        host.pending_dialog = Some(PendingPluginReply::Which(tx));
        host.cancel_pending();
        assert_eq!(rx.blocking_recv().unwrap(), None);
    }

    #[test]
    fn cancel_pending_when_empty_is_noop() {
        let mut host = PluginHostState::default();
        host.cancel_pending();
        assert!(host.pending_dialog.is_none());
    }
}
