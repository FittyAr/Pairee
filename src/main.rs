use anyhow::{Context, Result};
use simplelog::*;
use std::env;
use std::path::PathBuf;

mod app;
mod config;
mod fs;
mod keybindings;
mod terminal;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration TOML profiles
    let config =
        config::AppConfig::load_or_create().context("Failed to initialize config files")?;

    // 2. Setup application debug logger
    let log_path = config::paths::get_log_file_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(file) = std::fs::File::create(&log_path) {
        let _ = WriteLogger::init(LevelFilter::Debug, Config::default(), file);
    }

    log::info!("Starting NCRust application...");

    // 3. Resolve starting folders for panels
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let right_dir = current_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| current_dir.clone());

    // 4. Initialize context and state containers
    let context = app::AppContext::new(config);
    let state = app::AppState::new(current_dir, right_dir);

    // 5. Hand execution over to main loop
    app::run(context, state).await?;

    log::info!("NCRust exited cleanly.");
    Ok(())
}
