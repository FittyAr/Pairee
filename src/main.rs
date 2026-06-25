#![allow(clippy::all)]
use anyhow::{Context, Result};
use simplelog::*;
use std::env;
use std::path::PathBuf;

mod app;
mod config;
mod fs;
mod git;
mod keybindings;
mod terminal;
mod ui;
mod update;

#[tokio::main]
async fn main() -> Result<()> {
    // Install the rustls ring crypto provider for reqwest on non-Windows platforms
    #[cfg(not(target_os = "windows"))]
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Intercept elevated helper requests
    let args: Vec<String> = env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--elevated-helper") {
        if pos + 1 < args.len() {
            let temp_file = PathBuf::from(&args[pos + 1]);
            fs::elevated_helper::run_elevated_helper_loop(&temp_file)?;
        } else {
            anyhow::bail!("Missing temp file argument for --elevated-helper");
        }
        return Ok(());
    }

    // 0. Check if we need to spawn a standalone terminal window
    if terminal::standalone::check_and_launch_standalone().unwrap_or(false) {
        return Ok(());
    }

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

    log::info!("Starting Pairee application...");

    // 3. Resolve starting folders for panels
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let right_dir = current_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| current_dir.clone());

    // 4. Initialize context and state containers
    let context = app::AppContext::new(config);
    let mut state = app::AppState::new(current_dir, right_dir);
    state.case_sensitive_sort = context.config.settings.case_sensitive_sort;
    state.treat_digits_as_numbers = context.config.settings.treat_digits_as_numbers;
    state.sorting_collation = context.config.settings.sorting_collation.clone();
    state.req_admin_reading = context.config.settings.req_admin_reading;
    // Panel settings
    state.select_folders = context.config.settings.select_folders;
    state.sort_folder_names_by_extension = context.config.settings.sort_folder_names_by_extension;
    state.show_dotdot_in_root_folders = context.config.settings.show_dotdot_in_root_folders;
    state.disable_panel_update_object_count =
        context.config.settings.disable_panel_update_object_count;

    // 5. Launch background update check (if enabled)
    if context.config.settings.auto_update_check {
        let (tx, rx) = tokio::sync::oneshot::channel();
        update::checker::UpdateChecker::check_in_background(tx);
        state.update_check_rx = Some(rx);
        state.update_status = update::UpdateStatus::Checking;
    }

    // 6. Hand execution over to main loop
    app::run(context, state).await?;

    log::info!("Pairee exited cleanly.");
    Ok(())
}
