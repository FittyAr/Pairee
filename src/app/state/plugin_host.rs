use super::types::DevProgress;

/// Plugin-manager / developer-tools runtime channels.
#[derive(Default)]
pub struct PluginHostState {
    /// Drained each frame to update PluginMenu progress without blocking UI.
    pub dev_progress_rx: Option<tokio::sync::mpsc::UnboundedReceiver<DevProgress>>,
}
