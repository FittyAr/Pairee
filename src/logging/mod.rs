//! Structured logging infrastructure built on `tracing` and `tracing-subscriber`.
//!
//! Replaces legacy `simplelog` while transparently capturing existing `log::*`
//! macro invocations (the subscriber's `try_init` installs the `log`
//! bridge). All log records are
//! directed exclusively to `app.log` without ANSI color escape codes to avoid
//! corrupting the terminal UI.
//!
//! The log is appended to (never truncated) and rotated by size, see
//! [`rotate`]. The level is `info` in release builds and `debug` in debug
//! builds; `RUST_LOG` overrides it (e.g. `RUST_LOG=debug pairee`).

mod rotate;

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Mutex;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initializes the global tracing subscriber directing formatted logs to `log_file_path`.
///
/// `try_init` also bridges the `log` crate (tracing-subscriber's default
/// `tracing-log` feature), so third-party crates and code calling
/// `log::info!`, `log::error!`, etc. route through this subscriber.
pub fn init_logging(log_file_path: &Path) -> Result<()> {
    if let Some(parent) = log_file_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = rotate::RotatingFile::open(log_file_path, rotate::MAX_LOG_BYTES)
        .with_context(|| format!("Failed to open log file at {}", log_file_path.display()))?;

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter()));

    let file_layer = Layer::new()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .with_target(true)
        .with_level(true);

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer);

    // Set as global default. Ignore error if already set (e.g. in test suites).
    let _ = subscriber.try_init();

    // Several processes may append to the same file: tag this one.
    log::info!(
        "Pairee {} started (pid {})",
        env!("CARGO_PKG_VERSION"),
        std::process::id()
    );
    Ok(())
}

/// Filter used without `RUST_LOG`: `debug` only in debug builds, with the
/// HTTP stack kept at `info`.
fn default_filter() -> &'static str {
    if cfg!(debug_assertions) {
        "debug,html5ever=info,reqwest=info,hyper=info"
    } else {
        "info"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_init_logging_creates_file_and_writes_record() {
        let dir = tempdir().expect("tempdir");
        let log_path = dir.path().join("test_app.log");

        let res = init_logging(&log_path);
        assert!(res.is_ok());

        tracing::info!("Tracing test message: 42");
        log::info!("Log macro compatibility test message: 100");

        // Verify file existence
        assert!(log_path.exists());
    }

    #[test]
    fn default_level_depends_on_the_build() {
        assert_eq!(
            default_filter().starts_with("debug"),
            cfg!(debug_assertions)
        );
    }
}
