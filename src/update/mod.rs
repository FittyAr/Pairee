/// Auto-update system for Pairee.
///
/// This module handles:
/// - Detecting how Pairee was installed (tarball, deb, rpm, winget, etc.)
/// - Checking GitHub Releases for newer versions (with 1h cache)
/// - Downloading the correct asset for the current platform
/// - Applying the update (or notifying the user which package-manager command to run)

pub mod checker;
pub mod detect;
pub mod downloader;
pub mod installer;

pub use checker::UpdateInfo;

/// Status of an ongoing update operation.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    /// No update check has been started.
    Idle,
    /// Checking GitHub API for the latest release.
    Checking,
    /// Downloading the update asset (0.0–1.0 progress).
    Downloading(f32),
    /// The update has been downloaded and is being applied.
    Installing,
    /// The update was applied successfully.
    Done,
    /// An error occurred at some point.
    Error(String),
}
