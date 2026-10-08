//! Where a panel's entries come from: the location half of the panel
//! state (the path within it is `PanelState::current_path`).

use super::{Capabilities, LocalVfs, Vfs};
use crate::fs::ssh::SharedSshClient;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub enum PanelSource {
    /// The local filesystem.
    #[default]
    Local,
    /// An SFTP server.
    Remote(SharedSshClient),
}

impl PanelSource {
    /// The adapter serving this source.
    pub fn vfs(&self) -> Arc<dyn Vfs> {
        match self {
            Self::Local => Arc::new(LocalVfs),
            Self::Remote(client) => Arc::new(client.clone()),
        }
    }

    /// The SSH connection of a remote panel (transfer endpoints).
    pub fn ssh(&self) -> Option<&SharedSshClient> {
        match self {
            Self::Remote(client) => Some(client),
            _ => None,
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }

    pub fn capabilities(&self) -> Capabilities {
        self.vfs().capabilities()
    }
}
