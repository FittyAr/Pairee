mod cli;
mod dev_cli;
mod help;

use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

use crate::app;
use crate::config;
use crate::plugin;
use crate::terminal;
use crate::update;

/// CLI + TUI bootstrap. The binary only installs the tokio runtime around this function.
///
/// Logging starts first, so a failing subcommand, configuration load or
/// argument parse is recorded in `app.log` as well as printed.
pub async fn run() -> Result<()> {
    let _ = crate::logging::init_logging(&config::paths::get_log_file_path());
    let result = run_app().await;
    if let Err(e) = &result {
        log::error!("Pairee stopped with an error: {e:#}");
    }
    result
}

async fn run_app() -> Result<()> {
    // Install the rustls ring crypto provider for reqwest on non-Windows platforms
    #[cfg(not(target_os = "windows"))]
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Non-interactive subcommands (elevated helper, plugin and developer CLIs)
    let args: Vec<String> = env::args().collect();
    if cli::run_subcommand(&args).await? {
        return Ok(());
    }

    // 0. Check if we need to spawn a standalone terminal window
    if terminal::standalone::check_and_launch_standalone().unwrap_or(false) {
        return Ok(());
    }

    // 1. Load configuration TOML profiles
    let config =
        config::AppConfig::load_or_create().context("Failed to initialize config files")?;

    // 1b. Interactive arguments (after the language is loaded, so errors are localized)
    let launch = crate::launch_args::LaunchArgs::parse(args.into_iter().skip(1))
        .map_err(|e| anyhow::anyhow!(e.message()))?;
    if launch.no_mouse {
        terminal::mouse::disable_from_cli();
    }

    // Restore the terminal (raw mode, alternate screen, mouse capture) on panic
    // so the message is readable and the user's shell stays usable.
    terminal::panic_hook::install();

    log::info!("Starting Pairee application...");

    // 3-4. Context and state, with the panels of the restored session
    let context = app::AppContext::new(config);
    let mut state = initial_state(&context.config.settings, &launch.paths);

    // 5. Launch background update check (if enabled)
    if context.config.settings.auto_update_check {
        let (tx, rx) = tokio::sync::oneshot::channel();
        update::checker::UpdateChecker::check_in_background(tx);
        state.update.check_rx = Some(rx);
        state.update.status = update::UpdateStatus::Checking;
    }

    // 5.5. Initialize and load plugins
    plugin::PluginManager::init();
    if context.config.settings.plugins_enabled {
        plugin::PluginManager::load_all_plugins(&context.config).await;
    } else {
        log::info!("Plugins disabled by feature flag; skipping load");
    }

    // 6. Hand execution over to main loop
    let exit_dir = app::run(context, state).await?;
    report_exit_dir(&launch, exit_dir.as_deref());

    log::info!("Pairee exited cleanly.");
    Ok(())
}

/// Hands the focused panel's folder to a shell wrapper (`--cwd-file`,
/// `--print-cwd`) once the terminal is restored.
pub(crate) fn report_exit_dir(
    launch: &crate::launch_args::LaunchArgs,
    dir: Option<&std::path::Path>,
) {
    if let Some(file) = &launch.cwd_file
        && let Err(e) = app::session::cwd::write_cwd_file(file, dir)
    {
        log::warn!("Failed to write the cwd file {}: {e}", file.display());
    }
    if launch.print_cwd
        && let Some(dir) = dir
    {
        println!("{}", dir.display());
    }
}

/// The state at startup: sorting and panel options from the settings, then
/// the panels of `app::session::start` (setup defaults, last session,
/// command-line folders) with any notices queued for display.
pub(crate) fn initial_state(
    settings: &config::settings::Settings,
    cli_paths: &[PathBuf],
) -> app::AppState {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let right_dir = current_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| current_dir.clone());
    let mut state = app::AppState::new(current_dir, right_dir);
    state.apply_settings(settings);
    for notice in app::session::start(&mut state, settings, cli_paths) {
        state.dialogs.push(app::state::PopupType::info(notice));
    }
    state
}
