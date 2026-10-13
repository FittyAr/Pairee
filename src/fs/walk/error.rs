//! Why the walker could not return (or enter) an entry.

use crate::config::localization::t;
use std::path::{Path, PathBuf};

/// A problem met while walking. Only [`WalkError::Cancelled`] ends the
/// walk; after the others the walker goes on with the next entry.
#[derive(Debug)]
pub enum WalkError {
    /// The cancel callback asked to stop.
    Cancelled,
    /// A folder could not be listed.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A followed link leads back to a folder being walked.
    Cycle { path: PathBuf },
    /// The folder is deeper than the walk allows.
    TooDeep { path: PathBuf },
}

impl WalkError {
    /// The entry the error is about (`None` for a cancellation).
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Cancelled => None,
            Self::Io { path, .. } | Self::Cycle { path } | Self::TooDeep { path } => Some(path),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }
}

impl std::fmt::Display for WalkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let with_path = |key: &str, path: &Path| t(key).replacen("{}", &path.to_string_lossy(), 1);
        let text = match self {
            Self::Cancelled => t("walk_error_cancelled"),
            Self::Io { path, source } => {
                with_path("walk_error_unreadable", path).replacen("{}", &source.to_string(), 1)
            }
            Self::Cycle { path } => with_path("walk_error_cycle", path),
            Self::TooDeep { path } => with_path("walk_error_too_deep", path),
        };
        f.write_str(&text)
    }
}

impl std::error::Error for WalkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<WalkError> for std::io::Error {
    fn from(e: WalkError) -> Self {
        let kind = match &e {
            WalkError::Io { source, .. } => source.kind(),
            WalkError::Cancelled => std::io::ErrorKind::Interrupted,
            _ => std::io::ErrorKind::Other,
        };
        std::io::Error::new(kind, e.to_string())
    }
}
