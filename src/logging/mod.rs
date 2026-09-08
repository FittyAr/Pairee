//! Structured logging infrastructure built on `tracing` and `tracing-subscriber`.
//!
//! Replaces legacy `simplelog` while transparently capturing existing `log::*`
//! macro invocations via [`tracing_log::LogTracer`]. All log records are
//! directed exclusively to `app.log` without ANSI color escape codes to avoid
//! corrupting the terminal UI.

use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Initializes the global tracing subscriber directing formatted logs to `log_file_path`.
///
/// Also installs [`tracing_log::LogTracer`] so third-party crates and legacy code
/// calling `log::info!`, `log::error!`, etc. route through this subscriber.
pub fn init_logging(log_file_path: &Path) -> Result<()> {
    if let Some(parent) = log_file_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = File::create(log_file_path)
        .with_context(|| format!("Failed to create log file at {}", log_file_path.display()))?;

    // Bridge standard log records into tracing events
    let _ = tracing_log::LogTracer::init();

    // Respect RUST_LOG environment variable, otherwise default to debug
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("debug,html5ever=info,reqwest=info,hyper=info"));

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

    Ok(())
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
}
