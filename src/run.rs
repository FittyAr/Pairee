use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

use crate::app;
use crate::config;
use crate::fs;
use crate::git;
use crate::plugin;
use crate::terminal;
use crate::update;

/// CLI + TUI bootstrap. The binary only installs the tokio runtime around this function.
pub async fn run() -> Result<()> {
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

    // Intercept plugin and developer subcommands
    if args.len() > 1 {
        if args[1] == "plugin" {
            if args.len() > 2 {
                match args[2].as_str() {
                    "list" => {
                        plugin::updater::list_installed().await?;
                        return Ok(());
                    }
                    "search" => {
                        if args.len() > 3 {
                            plugin::updater::search(&args[3]).await?;
                        } else {
                            println!("Error: search requires a query string");
                        }
                        return Ok(());
                    }
                    "info" => {
                        if args.len() > 3 {
                            plugin::updater::show_info(&args[3]).await?;
                        } else {
                            println!("Error: info requires a plugin name");
                        }
                        return Ok(());
                    }
                    "install" | "add" => {
                        if args.len() > 3 {
                            for part in &args[3..] {
                                let (name, version) = if part.contains('@') {
                                    let split: Vec<&str> = part.split('@').collect();
                                    let clean_name = if split[0].ends_with(".pairee") {
                                        split[0].to_string()
                                    } else {
                                        format!("{}.pairee", split[0])
                                    };
                                    (clean_name, Some(split[1]))
                                } else {
                                    let clean_name = if part.ends_with(".pairee") {
                                        part.to_string()
                                    } else {
                                        format!("{}.pairee", part)
                                    };
                                    (clean_name, None)
                                };
                                if let Err(e) = plugin::updater::install(&name, version).await {
                                    println!("Error installing plugin '{}': {:?}", name, e);
                                }
                            }
                        } else {
                            println!("Error: install command requires at least one plugin name");
                        }
                        return Ok(());
                    }
                    "remove" => {
                        if args.len() > 3 {
                            plugin::updater::remove(&args[3])?;
                        } else {
                            println!("Error: remove requires a plugin name");
                        }
                        return Ok(());
                    }
                    "pin" => {
                        if args.len() > 3 {
                            plugin::updater::pin(&args[3], true)?;
                        } else {
                            println!("Error: pin requires a plugin name");
                        }
                        return Ok(());
                    }
                    "unpin" => {
                        if args.len() > 3 {
                            plugin::updater::pin(&args[3], false)?;
                        } else {
                            println!("Error: unpin requires a plugin name");
                        }
                        return Ok(());
                    }
                    "verify" => {
                        plugin::updater::verify().await?;
                        return Ok(());
                    }
                    "check-updates" => {
                        plugin::updater::check_updates().await?;
                        return Ok(());
                    }
                    "update" => {
                        let name = if args.len() > 3 {
                            Some(args[3].as_str())
                        } else {
                            None
                        };
                        plugin::updater::update(name).await?;
                        return Ok(());
                    }
                    _ => {
                        println!(
                            "Unknown plugin command. Available: list, search, info, install, add, remove, pin, unpin, verify, check-updates, update"
                        );
                    }
                }
            } else {
                println!(
                    "Plugin CLI usage: pairee plugin [list|search|info|install|add|remove|pin|unpin|verify|check-updates|update]"
                );
            }
            return Ok(());
        } else if args[1] == "developer" {
            if args.len() > 2 {
                match args[2].as_str() {
                    "init" => {
                        if args.len() > 3 {
                            let name = &args[3];
                            println!("Enter plugin description:");
                            let mut desc = String::new();
                            std::io::stdin().read_line(&mut desc)?;
                            let desc = desc.trim().to_string();

                            println!("Enter plugin author:");
                            let mut author = String::new();
                            std::io::stdin().read_line(&mut author)?;
                            let author = author.trim().to_string();

                            plugin::developer_tool::init(name, &desc, &author, true)?;
                        } else {
                            println!("Error: init requires a plugin name");
                        }
                        return Ok(());
                    }
                    "lint" => {
                        plugin::developer_tool::lint()?;
                        return Ok(());
                    }
                    "package" => {
                        plugin::developer_tool::package()?;
                        return Ok(());
                    }
                    "submit" => {
                        plugin::developer_tool::submit().await?;
                        return Ok(());
                    }
                    _ => {
                        println!(
                            "Unknown developer command. Available: init, lint, package, submit"
                        );
                    }
                }
            } else {
                println!("Developer CLI usage: pairee developer [init|lint|package|submit]");
            }
            return Ok(());
        }
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

    // 2. Setup application debug logger (structured tracing)
    let log_path = config::paths::get_log_file_path();
    let _ = crate::logging::init_logging(&log_path);

    // Restore the terminal (raw mode, alternate screen, mouse capture) on panic
    // so the message is readable and the user's shell stays usable.
    terminal::panic_hook::install();

    log::info!("Starting Pairee application...");
    git::unused_keepalive();

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
        plugin::PluginManager::load_all_plugins(&context).await;
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
fn report_exit_dir(launch: &crate::launch_args::LaunchArgs, dir: Option<&std::path::Path>) {
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
fn initial_state(settings: &config::settings::Settings, cli_paths: &[PathBuf]) -> app::AppState {
    let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let right_dir = current_dir
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| current_dir.clone());
    let mut state = app::AppState::new(current_dir, right_dir);
    state.case_sensitive_sort = settings.case_sensitive_sort;
    state.treat_digits_as_numbers = settings.treat_digits_as_numbers;
    state.sorting_collation = settings.sorting_collation.clone();
    state.req_admin_reading = settings.req_admin_reading;
    state.select_folders = settings.select_folders;
    state.sort_folder_names_by_extension = settings.sort_folder_names_by_extension;
    state.show_dotdot_in_root_folders = settings.show_dotdot_in_root_folders;
    state.disable_panel_update_object_count = settings.disable_panel_update_object_count;
    for notice in app::session::start(&mut state, settings, cli_paths) {
        state.dialogs.push(app::state::PopupType::Info(notice));
    }
    state
}
